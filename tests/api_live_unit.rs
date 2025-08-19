//! Live API test for unit preference. Run with: `cargo test --features online -- --nocapture`
#![cfg(feature = "online")]

use std::fs;
use world_bank_data_rust::viz::LegendMode;
use world_bank_data_rust::{Client, DateSpec, viz};

#[test]
fn api_provided_unit_appears_in_chart() {
    // Fetch SP.POP.TOTL for DEU for a short range (should have "Number" as unit)
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

    assert!(!pts.is_empty(), "Should fetch some data points");

    // Check that we got unit values from the API
    let has_units = pts
        .iter()
        .any(|p| p.unit.is_some() && !p.unit.as_ref().unwrap().is_empty());
    assert!(has_units, "Expected API to provide unit values");

    // Render chart to a temporary SVG
    let tmp_svg = std::env::temp_dir().join("wbd_unit_test.svg");
    viz::plot_chart(
        &pts,
        &tmp_svg,
        800,
        480,
        "en",
        LegendMode::Right,
        "Unit Test Chart",
        viz::PlotKind::LinePoints,
        0.3,
    )
    .unwrap();

    // Read the SVG and check that it contains the API-provided unit
    let svg_content = fs::read_to_string(&tmp_svg).unwrap();

    // SP.POP.TOTL should have "Number" as unit based on World Bank API
    // Allow for case-insensitive matching and potential scale suffixes like "(millions)"
    let svg_lower = svg_content.to_lowercase();
    assert!(
        svg_lower.contains("number") || svg_lower.contains("population"),
        "SVG should contain API-provided unit. Content: {}",
        svg_content
    );

    // Clean up
    fs::remove_file(&tmp_svg).ok();
}
