/// Synchronous client for the **World Bank Indicators API (v2)**.
///
/// This module provides access to both observation data from the `country/{codes}/indicator/{codes}` 
/// endpoint and indicator metadata from the `indicator/{id}` endpoint. Results are returned as tidy 
/// `models::DataPoint` rows with automatic pagination handling.
///
/// ### Endpoints supported
/// - **Observations**: `country/{codes}/indicator/{codes}` - returns time series data
/// - **Indicator metadata**: `indicator/{id}` - returns metadata including units
///
/// ### Notes
/// - The API sometimes serializes `per_page` as a **string**; we accept both string/number.
/// - When requesting **multiple indicators** at once, the API requires a `source` parameter
///   (e.g., `source=2` for WDI). Pass it via `Client::fetch(..., Some(2))`.
/// - Network timeouts use a sane default (30s) and can be adjusted by editing the client builder.
/// - Use `populate_units_from_metadata()` to enhance DataPoints with proper unit information.
///
///
/// Typical usage:
/// ```no_run
/// # use world_bank_data_rust::{Client, DateSpec};
/// let client = Client::default();
/// let mut rows = client.fetch(
///     &["DEU".into()],
///     &["SP.POP.TOTL".into()],
///     Some(DateSpec::Year(2020)),
///     None,
/// )?;
/// 
/// // Optionally populate missing units from metadata
/// client.populate_units_from_metadata(&mut rows)?;
/// # Ok::<(), anyhow::Error>(())
/// ```
use crate::models::{DataPoint, DateSpec, Entry, Meta, IndicatorMetadata};
use anyhow::{Context, Result, bail};
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC};
use reqwest::blocking::Client as HttpClient;
use reqwest::redirect::Policy;
use serde_json::Value;
use std::time::Duration;

/// Fetch indicator observations.
///
/// ### Arguments
/// - `countries`: ISO2/ISO3 country codes or aggregate codes (`"DEU"`, `"USA"`, `"EUU"`…).
///   Multiple codes are allowed; they are joined for the API (e.g., `"DEU;USA"`).
/// - `indicators`: Indicator IDs (`"SP.POP.TOTL"`, …). Multiple allowed.
/// - `date`: A single year (`Year(2020)`) or an inclusive range (`Range { start, end }`).
/// - `source`: Optional numeric source id (e.g., `2` for WDI). **Required** by the API when
///   requesting multiple indicators.
///
/// ### Returns
/// A `Vec<models::DataPoint>` where one row equals one observation (country, indicator, year).
///
/// ### Errors
/// - Network/HTTP error
/// - JSON decoding error
/// - API-level error payload (surfaced as an error)
///
/// ### Example
/// ```no_run
/// # use world_bank_data_rust::{Client, DateSpec};
/// let cli = Client::default();
/// let data = cli.fetch(
///     &["DEU".into(), "USA".into()],
///     &["SP.POP.TOTL".into()],
///     Some(DateSpec::Range { start: 2015, end: 2020 }),
///     None,
/// )?;
/// # Ok::<(), anyhow::Error>(())
/// ```

#[derive(Debug, Clone)]
pub struct Client {
    pub base_url: String,
    http: HttpClient,
}

impl Default for Client {
    fn default() -> Self {
        let http = HttpClient::builder()
            .timeout(Duration::from_secs(30)) // total request timeout
            .connect_timeout(Duration::from_secs(10)) // connect timeout
            .redirect(Policy::limited(5)) // cap redirects
            .user_agent(concat!("world_bank_data_rust/", env!("CARGO_PKG_VERSION"))) // set user agent
            .build()
            .expect("reqwest client build");
        Self {
            base_url: "https://api.worldbank.org/v2".into(),
            http,
        }
    }
}

// Allow -, _, . unescaped in codes (common for indicator ids)
const SAFE: &AsciiSet = &NON_ALPHANUMERIC.remove(b'-').remove(b'_').remove(b'.');

fn enc_join<'a>(parts: impl IntoIterator<Item = &'a str>) -> String {
    parts
        .into_iter()
        .map(|s| percent_encoding::utf8_percent_encode(s.trim(), SAFE).to_string())
        .collect::<Vec<_>>()
        .join(";")
}

impl Client {
    /// Fetch indicator observations.
    ///
    /// - `countries`: ISO2 (e.g., "DE") or ISO3 (e.g., "DEU") or aggregates (e.g., "EUU"). Multiple accepted.
    /// - `indicators`: e.g., "SP.POP.TOTL". Multiple accepted.
    /// - `date`: A single year or inclusive range.
    /// - `source`: Optional numeric source id (e.g., 2 for WDI). Required by API when querying *multiple* indicators.
    pub fn fetch(
        &self,
        countries: &[String],
        indicators: &[String],
        date: Option<DateSpec>,
        source: Option<u32>,
    ) -> Result<Vec<DataPoint>> {
        if countries.is_empty() {
            bail!("at least one country/region code required");
        }
        if indicators.is_empty() {
            bail!("at least one indicator code required");
        }

        let country_spec = enc_join(countries.iter().map(|s| s.as_str()));
        let indicator_spec = enc_join(indicators.iter().map(|s| s.as_str()));

        let mut url = format!(
            "{}/country/{}/indicator/{}?format=json&per_page=1000",
            self.base_url, country_spec, indicator_spec
        );
        if let Some(d) = date {
            url.push_str(&format!("&date={}", d.to_query_param()));
        }
        if let Some(s) = source {
            url.push_str(&format!("&source={}", s));
        }

        // Small retry for transient failures (5xx / network errors)
        let get_json = |u: &str| -> Result<Value> {
            let mut last_err: Option<anyhow::Error> = None;
            for backoff_ms in [100u64, 300, 700] {
                match self.http.get(u).send() {
                    Ok(r) if r.status().is_success() => {
                        return r.json().context("decode json");
                    }
                    Ok(r) if r.status().is_server_error() => { /* retry */ }
                    Ok(r) => bail!("request failed with HTTP {}", r.status()),
                    Err(e) => last_err = Some(e.into()),
                }
                std::thread::sleep(Duration::from_millis(backoff_ms));
            }
            bail!("network error: {:?}", last_err);
        };

        // Safety cap to avoid pathological jobs
        let max_pages = 1000u32;

        // Paginate until we retrieved all pages.
        let mut page = 1u32;
        let mut out: Vec<DataPoint> = Vec::new();
        loop {
            let page_url = format!("{}&page={}", url, page);
            if page > max_pages {
                bail!("page limit exceeded ({})", max_pages);
            }
            let v: Value = get_json(&page_url).with_context(|| format!("GET {}", page_url))?;

            // The API returns an array: [Meta, [Entry, ...]] or a "message" object in position 0 on error.
            let arr = v.as_array().ok_or_else(|| {
                anyhow::anyhow!("unexpected response shape: not a top-level array")
            })?;
            if arr.is_empty() {
                bail!("unexpected response: empty array");
            }

            // If first element has "message", surface API error.
            if arr[0].get("message").is_some() {
                bail!("world bank api error: {}", arr[0]);
            }

            let meta: Meta = serde_json::from_value(arr[0].clone()).context("parse meta")?;
            let entries: Vec<Entry> = if arr.len() > 1 {
                serde_json::from_value(arr[1].clone()).context("parse entries")?
            } else {
                vec![]
            };

            out.extend(entries.into_iter().map(DataPoint::from));

            let total_pages = meta.pages;
            if page >= total_pages {
                break;
            }
            page += 1;
        }
        Ok(out)
    }

    /// Fetch indicator metadata to get unit information.
    ///
    /// ### Arguments
    /// - `indicator_id`: Single indicator ID (e.g., "SP.POP.TOTL")
    ///
    /// ### Returns
    /// `IndicatorMetadata` containing id, name, and unit information
    ///
    /// ### Errors
    /// - Network/HTTP error
    /// - JSON decoding error
    /// - API-level error payload
    ///
    /// ### Example
    /// ```no_run
    /// # use world_bank_data_rust::Client;
    /// let cli = Client::default();
    /// let metadata = cli.fetch_indicator_metadata("SP.POP.TOTL")?;
    /// println!("Unit: {:?}", metadata.unit);
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    pub fn fetch_indicator_metadata(&self, indicator_id: &str) -> Result<IndicatorMetadata> {
        let encoded_id = percent_encoding::utf8_percent_encode(indicator_id.trim(), SAFE).to_string();
        let url = format!("{}/indicator/{}?format=json", self.base_url, encoded_id);

        // Use the same retry logic as fetch method
        let get_json = |u: &str| -> Result<Value> {
            let mut last_err: Option<anyhow::Error> = None;
            for backoff_ms in [100u64, 300, 700] {
                match self.http.get(u).send() {
                    Ok(r) if r.status().is_success() => {
                        return r.json().context("decode json");
                    }
                    Ok(r) if r.status().is_server_error() => { /* retry */ }
                    Ok(r) => bail!("request failed with HTTP {}", r.status()),
                    Err(e) => last_err = Some(e.into()),
                }
                std::thread::sleep(Duration::from_millis(backoff_ms));
            }
            bail!("network error: {:?}", last_err);
        };

        let v: Value = get_json(&url).with_context(|| format!("GET {}", url))?;

        // The API returns an array: [Meta, [IndicatorMetadata, ...]]
        let arr = v.as_array().ok_or_else(|| {
            anyhow::anyhow!("unexpected response shape: not a top-level array")
        })?;
        if arr.is_empty() {
            bail!("unexpected response: empty array");
        }

        // If first element has "message", surface API error.
        if arr[0].get("message").is_some() {
            bail!("world bank api error: {}", arr[0]);
        }

        // Parse the data array (position 1)
        let indicators: Vec<IndicatorMetadata> = if arr.len() > 1 {
            serde_json::from_value(arr[1].clone()).context("parse indicator metadata")?
        } else {
            vec![]
        };

        // Return the first indicator (should be the one we requested)
        indicators.into_iter().next()
            .ok_or_else(|| anyhow::anyhow!("no indicator metadata found for {}", indicator_id))
    }

    /// Populate DataPoint units from indicator metadata when missing.
    /// 
    /// This method fetches indicator metadata for any indicators that have
    /// missing or empty units in the provided DataPoints and updates them.
    ///
    /// ### Arguments
    /// - `points`: Mutable reference to DataPoints to update
    ///
    /// ### Returns
    /// `Ok(())` on success, or error if metadata fetching fails
    ///
    /// ### Example
    /// ```no_run
    /// # use world_bank_data_rust::Client;
    /// let cli = Client::default();
    /// let mut points = cli.fetch(&["DEU".into()], &["SP.POP.TOTL".into()], None, None)?;
    /// cli.populate_units_from_metadata(&mut points)?;
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    pub fn populate_units_from_metadata(&self, points: &mut [DataPoint]) -> Result<()> {
        use std::collections::{HashMap, HashSet};
        
        // Find all indicators that need unit information
        let mut indicators_needing_units: HashSet<String> = HashSet::new();
        for point in points.iter() {
            if point.unit.is_none() || point.unit.as_ref().map_or(true, |u| u.trim().is_empty()) {
                indicators_needing_units.insert(point.indicator_id.clone());
            }
        }
        
        // Fetch metadata for indicators that need it
        let mut metadata_cache: HashMap<String, Option<String>> = HashMap::new();
        for indicator_id in indicators_needing_units {
            match self.fetch_indicator_metadata(&indicator_id) {
                Ok(metadata) => {
                    metadata_cache.insert(indicator_id, metadata.unit);
                }
                Err(_) => {
                    // If metadata fetch fails, leave the unit as-is
                    // This ensures the method doesn't fail if some indicators
                    // don't have metadata available
                    metadata_cache.insert(indicator_id, None);
                }
            }
        }
        
        // Update points with fetched metadata
        for point in points.iter_mut() {
            if point.unit.is_none() || point.unit.as_ref().map_or(true, |u| u.trim().is_empty()) {
                if let Some(metadata_unit) = metadata_cache.get(&point.indicator_id) {
                    point.unit = metadata_unit.clone();
                }
            }
        }
        
        Ok(())
    }
}
