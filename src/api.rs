/// Synchronous client for the **World Bank Indicators API (v2)**.
///
/// This module focuses on the `country/{codes}/indicator/{codes}` endpoint and returns
/// results as tidy `models::DataPoint` rows. Pagination is handled automatically.
///
/// ### Notes
/// - The API sometimes serializes `per_page` as a **string**; we accept both string/number.
/// - When requesting **multiple indicators** at once, the API requires a `source` parameter
///   (e.g., `source=2` for WDI). Pass it via `Client::fetch(..., Some(2))`.
/// - Network timeouts use a sane default (30s) and can be adjusted by editing the client builder.
///
///
/// Typical usage:
/// ```no_run
/// # use wbi_rs::{Client, DateSpec};
/// let client = Client::default();
/// let rows = client.fetch(
///     &["DEU".into()],
///     &["SP.POP.TOTL".into()],
///     Some(DateSpec::Year(2020)),
///     None,
/// )?;
/// # Ok::<(), anyhow::Error>(())
/// ```
use crate::models::{DataPoint, DateSpec, Entry, IndicatorMeta, Meta};
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
/// # use wbi_rs::{Client, DateSpec};
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
            .user_agent(concat!("wbi_rs/", env!("CARGO_PKG_VERSION"))) // set user agent
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
    /// Fetch units from the World Bank indicator endpoint for the given indicators.
    ///
    /// Returns a map from indicator ID to unit string. Missing indicators or those
    /// without units will not be present in the returned HashMap.
    ///
    /// ### Arguments
    /// - `indicators`: Indicator IDs to fetch metadata for (e.g., `["SP.POP.TOTL"]`).
    ///
    /// ### Example
    /// ```no_run
    /// # use wbi_rs::Client;
    /// let cli = Client::default();
    /// let units = cli.fetch_indicator_units(&["SP.POP.TOTL".into()])?;
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    pub fn fetch_indicator_units(
        &self,
        indicators: &[String],
    ) -> Result<std::collections::HashMap<String, String>> {
        use std::collections::HashMap;

        if indicators.is_empty() {
            return Ok(HashMap::new());
        }

        let indicator_spec = enc_join(indicators.iter().map(|s| s.as_str()));
        let url = format!(
            "{}/indicator/{}?format=json&per_page=1000",
            self.base_url, indicator_spec
        );

        // Use the same retry logic as the main fetch method
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

        // Parse the response (same structure as data endpoint: [Meta, [IndicatorMeta, ...]])
        let arr = v
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("unexpected response shape: not a top-level array"))?;
        if arr.is_empty() {
            bail!("unexpected response: empty array");
        }

        // Check for API error
        if arr[0].get("message").is_some() {
            bail!("world bank api error: {}", arr[0]);
        }

        let indicators_data: Vec<IndicatorMeta> = if arr.len() > 1 {
            serde_json::from_value(arr[1].clone()).context("parse indicator metadata")?
        } else {
            vec![]
        };

        // Build map from ID to unit
        let mut result = HashMap::new();
        for meta in indicators_data {
            if let Some(unit) = meta.unit {
                if !unit.trim().is_empty() {
                    result.insert(meta.id, unit);
                }
            }
        }

        Ok(result)
    }

    /// Fetch indicator observations.
    ///
    /// - `countries`: ISO2 (e.g., "DE") or ISO3 (e.g., "DEU") or aggregates (e.g., "EUU"). Multiple accepted.
    /// - `indicators`: e.g., "SP.POP.TOTL". Multiple accepted.
    /// - `date`: A single year or inclusive range.
    /// - `source`: Optional numeric source id (e.g., 2 for WDI). Required by the World Bank API
    ///   for efficient single-call multi-indicator requests. When `source` is `None` and multiple
    ///   indicators are requested, this method automatically falls back to individual requests
    ///   per indicator and merges the results.
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

        // Multi-indicator fallback: if multiple indicators without source,
        // fetch each indicator separately and merge results
        if indicators.len() > 1 && source.is_none() {
            let mut all_points = Vec::new();
            for indicator in indicators {
                let points = self.fetch(countries, &[indicator.clone()], date.clone(), None)?;
                all_points.extend(points);
            }
            return Ok(all_points);
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

        // Unit enrichment: if any DataPoints lack units, try to fetch from indicator metadata
        let needs_enrichment = out.iter().any(|p| {
            p.unit.is_none()
                || p.unit
                    .as_ref()
                    .map(|u| u.trim().is_empty())
                    .unwrap_or(false)
        });

        if needs_enrichment {
            // Fetch indicator metadata to get units
            match self.fetch_indicator_units(indicators) {
                Ok(indicator_units) => {
                    // Enrich DataPoints that lack units
                    for point in &mut out {
                        if point.unit.is_none()
                            || point
                                .unit
                                .as_ref()
                                .map(|u| u.trim().is_empty())
                                .unwrap_or(false)
                        {
                            if let Some(unit) = indicator_units.get(&point.indicator_id) {
                                point.unit = Some(unit.clone());
                            }
                        }
                    }
                }
                Err(_) => {
                    // If indicator metadata fetch fails, continue without enrichment
                    // This ensures that the main data fetch doesn't fail due to metadata issues
                }
            }
        }

        Ok(out)
    }
}
