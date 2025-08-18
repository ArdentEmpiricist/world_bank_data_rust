//! Live API tests. Run with: `cargo test --features online -- --nocapture`
#![cfg(feature = "online")]

use world_bank_data_rust::{Client, DateSpec};

#[test]
fn fetch_small_range() {
    let cli = Client::default();
    let pts = cli
        .fetch(
            &["DEU".into()],
            &["SP.POP.TOTL".into()],
            Some(DateSpec::Range {
                start: 2019,
                end: 2020,
            }),
            None,
        )
        .unwrap();
    assert!(!pts.is_empty());
    assert!(pts.iter().all(|p| p.country_iso3 == "DEU"));
    assert!(pts.iter().all(|p| p.year >= 2019 && p.year <= 2020));
}

#[test]
fn fetch_multiple_indicators_with_source() {
    let cli = Client::default();
    let pts = cli
        .fetch(
            &["DEU".into()],
            &["SP.POP.TOTL".into(), "NY.GDP.MKTP.CD".into()],
            Some(DateSpec::Range {
                start: 2019,
                end: 2020,
            }),
            Some(2), // WDI source id; required by the World Bank API when querying multiple indicators
        )
        .unwrap();

    assert!(!pts.is_empty());

    // All rows should be for DEU and within the requested range.
    assert!(pts.iter().all(|p| p.country_iso3 == "DEU"));
    assert!(pts.iter().all(|p| p.year >= 2019 && p.year <= 2020));

    // Ensure both indicators are present.
    use std::collections::BTreeSet;
    let ind_ids: BTreeSet<&str> = pts.iter().map(|p| p.indicator_id.as_str()).collect();
    assert!(ind_ids.contains("SP.POP.TOTL"));
    assert!(ind_ids.contains("NY.GDP.MKTP.CD"));
}
