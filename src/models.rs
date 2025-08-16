use serde::{Deserialize, Serialize};

/// How to specify dates in API queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DateSpec {
    /// Single year like 2020
    Year(i32),
    /// Inclusive range like 2000..=2020
    Range { start: i32, end: i32 },
}

impl DateSpec {
    pub fn to_query_param(&self) -> String {
        match *self {
            DateSpec::Year(y) => y.to_string(),
            DateSpec::Range { start, end } => format!("{}:{}", start, end),
        }
    }
}

/// Metadata section returned by the API (position 0).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Meta {
    pub page: u32,
    pub pages: u32,
    /// Some responses encode `per_page` as a string, others as a number.
    /// Accept both and normalize to `u32`.
    #[serde(deserialize_with = "de_u32_from_string_or_number")]
    pub per_page: u32,
    pub total: u32,
}

/// Serde helper: parse `u32` from either a JSON number or a string.
fn de_u32_from_string_or_number<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{self, Visitor};
    struct U32Visitor;

    impl<'de> Visitor<'de> for U32Visitor {
        type Value = u32;

        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(f, "a string or integer representing a non-negative number")
        }

        fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(v as u32)
        }

        fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            if v < 0 {
                return Err(E::custom("negative value for u32"));
            }
            Ok(v as u32)
        }

        fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            s.parse::<u32>().map_err(E::custom)
        }
    }

    deserializer.deserialize_any(U32Visitor)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeName {
    pub id: String,
    pub value: String,
}

/// Raw entry from the API (position 1 array).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub indicator: CodeName,
    pub country: CodeName,
    pub countryiso3code: String,
    pub date: String,
    pub value: Option<f64>,
    pub unit: Option<String>,
    #[serde(rename = "obs_status")]
    pub obs_status: Option<String>,
    pub decimal: Option<i32>,
}

/// Tidy structure used by this crate (one row = one observation).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DataPoint {
    pub indicator_id: String,
    pub indicator_name: String,
    pub country_id: String, // typically ISO2
    pub country_name: String,
    pub country_iso3: String,
    pub year: i32,
    pub value: Option<f64>,
    pub unit: Option<String>,
    pub obs_status: Option<String>,
    pub decimal: Option<i32>,
}

impl From<Entry> for DataPoint {
    fn from(e: Entry) -> Self {
        let year = e.date.parse::<i32>().unwrap_or(0);
        Self {
            indicator_id: e.indicator.id,
            indicator_name: e.indicator.value,
            country_id: e.country.id,
            country_name: e.country.value,
            country_iso3: e.countryiso3code,
            year,
            value: e.value,
            unit: e.unit,
            obs_status: e.obs_status,
            decimal: e.decimal,
        }
    }
}

/// Grouping key used in stats and plotting.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GroupKey {
    pub indicator_id: String,
    pub country_iso3: String,
}
