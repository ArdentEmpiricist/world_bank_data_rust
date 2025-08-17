//! Live API tests. Run with: `cargo test --features online -- --nocapture`
#![cfg(feature = "online")]

use world_bank_data_rust::{Client, DateSpec};

#[test]
fn fetch_small_range() {
    let cli = Client::default();
    let pts = cli.fetch(&["DEU".into()], &["SP.POP.TOTL".into()], Some(DateSpec::Range{ start: 2019, end: 2020 }), None).unwrap();
    assert!(!pts.is_empty());
    assert!(pts.iter().all(|p| p.country_iso3 == "DEU"));
    assert!(pts.iter().all(|p| p.year >= 2019 && p.year <= 2020));
}