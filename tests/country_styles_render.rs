use std::fs;
use std::path::PathBuf;

use wbi_rs::models::DataPoint;
use wbi_rs::viz::{self, LegendMode, PlotKind};

fn make_tmp_svg(name: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    p.push(format!("wbi_test_{}_{}.svg", name, ts));
    p
}

fn points_two_china_series() -> Vec<DataPoint> {
    // Two indicators for the same country (CHN), plus one for USA
    let mut v = Vec::new();
    for year in 2015..=2017 {
        v.push(DataPoint {
            country_name: "China".to_string(),
            country_iso3: "CHN".to_string(),
            country_id: "CHN".to_string(),
            indicator_name: "Population Total".to_string(),
            indicator_id: "SP.POP.TOTL".to_string(),
            year,
            value: Some(1_370_000_000.0 + (year as f64 - 2015.0) * 1_000_000.0),
            unit: None,
            obs_status: None,
            decimal: None,
        });
        v.push(DataPoint {
            country_name: "China".to_string(),
            country_iso3: "CHN".to_string(),
            country_id: "CHN".to_string(),
            indicator_name: "Population Female %".to_string(),
            indicator_id: "SP.POP.TOTL.FE.IN".to_string(),
            year,
            value: Some(680_000_000.0 + (year as f64 - 2015.0) * 500_000.0),
            unit: None,
            obs_status: None,
            decimal: None,
        });
        v.push(DataPoint {
            country_name: "United States".to_string(),
            country_iso3: "USA".to_string(),
            country_id: "USA".to_string(),
            indicator_name: "Population Total".to_string(),
            indicator_id: "SP.POP.TOTL".to_string(),
            year,
            value: Some(320_000_000.0 + (year as f64 - 2015.0) * 1_000_000.0),
            unit: None,
            obs_status: None,
            decimal: None,
        });
    }
    v
}

#[test]
fn country_styles_linepoints_draws_markers() {
    let svg_path = make_tmp_svg("markers");
    let pts = points_two_china_series();

    viz::plot_chart(
        &pts,
        &svg_path,
        800,
        480,
        "en",
        LegendMode::Right,
        "Markers Test",
        PlotKind::LinePoints,
        0.3,
        Some(true), // enable country styles
    )
    .expect("plot should be created");

    let s = fs::read_to_string(&svg_path).expect("read svg");
    // Expect some marker glyphs present (at least circles; ideally other shapes too)
    let has_any_markers = s.contains("<circle") || s.contains("<rect") || s.contains("<polygon");
    assert!(has_any_markers, "expected marker glyphs in SVG");

    let _ = fs::remove_file(&svg_path);
}

#[test]
fn country_styles_legend_dedups_country_right() {
    let svg_path = make_tmp_svg("legend_dedup");
    let pts = points_two_china_series();

    viz::plot_chart(
        &pts,
        &svg_path,
        800,
        480,
        "en",
        LegendMode::Right,
        "Legend Dedup Test",
        PlotKind::LinePoints,
        0.3,
        Some(true), // enable country styles
    )
    .expect("plot should be created");

    let s = fs::read_to_string(&svg_path).expect("read svg");
    let count_china = s.matches("China").count();
    assert!(
        count_china == 1,
        "expected exactly one 'China' in legend, got {}",
        count_china
    );

    let _ = fs::remove_file(&svg_path);
}
