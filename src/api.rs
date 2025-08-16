use crate::models::{DataPoint, DateSpec, Entry, Meta};
use anyhow::{bail, Context, Result};
use reqwest::blocking::Client as HttpClient;
use serde_json::Value;
use std::time::Duration;

/// A small sync client over the World Bank Indicators API (v2).
///
/// Docs: https://datahelpdesk.worldbank.org/knowledgebase/articles/889392-about-the-indicators-api-documentation
///
/// This client focuses on the `/v2/country/{codes}/indicator/{codes}` endpoint.
#[derive(Debug, Clone)]
pub struct Client {
    pub base_url: String,
    http: HttpClient,
}

impl Default for Client {
    fn default() -> Self {
        let http = HttpClient::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("reqwest client build");
        Self {
            base_url: "https://api.worldbank.org/v2".into(),
            http,
        }
    }
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
        if countries.is_empty() { bail!("at least one country/region code required"); }
        if indicators.is_empty() { bail!("at least one indicator code required"); }

        let country_spec = countries.join(";");
        let indicator_spec = indicators.join(";");

        let mut url = format!("{}/country/{}/indicator/{}?format=json&per_page=1000",
            self.base_url, country_spec, indicator_spec);
        if let Some(d) = date {
            url.push_str(&format!("&date={}", d.to_query_param()));
        }
        if let Some(s) = source {
            url.push_str(&format!("&source={}", s));
        }

        // Paginate until we retrieved all pages.
        let mut page = 1u32;
        let mut out: Vec<DataPoint> = Vec::new();
        loop {
            let page_url = format!("{}&page={}", url, page);
            let resp = self.http.get(&page_url).send()
                .with_context(|| format!("GET {}", page_url))?;
            if !resp.status().is_success() {
                bail!("request failed with HTTP {}", resp.status());
            }
            let v: Value = resp.json().context("decode json")?;

            // The API returns an array: [Meta, [Entry, ...]] or a "message" object in position 0 on error.
            let arr = v.as_array().ok_or_else(|| anyhow::anyhow!("unexpected response shape: not a top-level array"))?;
            if arr.is_empty() {
                bail!("unexpected response: empty array");
            }

            // If first element has "message", surface API error.
            if arr[0].get("message").is_some() {
                bail!("world bank api error: {}", arr[0]);
            }

            let meta: Meta = serde_json::from_value(arr[0].clone())
                .context("parse meta")?;
            let entries: Vec<Entry> = if arr.len() > 1 {
                serde_json::from_value(arr[1].clone()).context("parse entries")?
            } else {
                vec![]
            };

            out.extend(entries.into_iter().map(DataPoint::from));

            let total_pages = meta.pages;
            if page >= total_pages { break; }
            page += 1;
        }
        Ok(out)
    }
}