use std::fs;
use std::path::PathBuf;
use world_bank_data_rust::models::DataPoint;
use world_bank_data_rust::viz::{self, LegendMode, PlotKind};

fn sample_points() -> Vec<DataPoint> {
    let mut out = Vec::new();
    // Series 1: DEU
    for (y, v) in [(2019, 1.0), (2020, 2.0), (2021, 3.0)] {
        out.push(DataPoint {
            indicator_id: "X".into(),
            indicator_name: "Demo Indicator".into(),
            country_id: "DE".into(),
            country_name: "Germany".into(),
            country_iso3: "DEU".into(),
            year: y,
            value: Some(v),
            unit: None,
            obs_status: None,
            decimal: None,
        });
    }
    // Series 2: USA
    for (y, v) in [(2019, 2.0), (2020, 2.5), (2021, 3.5)] {
        out.push(DataPoint {
            indicator_id: "X".into(),
            indicator_name: "Demo Indicator".into(),
            country_id: "US".into(),
            country_name: "United States".into(),
            country_iso3: "USA".into(),
            year: y,
            value: Some(v),
            unit: None,
            obs_status: None,
            decimal: None,
        });
    }
    out
}

fn write_and_check<F: Fn(&PathBuf)>(maker: F, name: &str) {
    let tmp = std::env::temp_dir();
    let path: PathBuf = tmp.join(format!("wbd_viz_{}.svg", name));
    maker(&path);
    let meta = fs::metadata(&path).expect("file created");
    assert!(meta.len() > 0, "svg has content");
    fs::remove_file(&path).ok();
}

#[test]
fn plot_kinds_produce_files() {
    let points = sample_points();
    let kinds = [
        PlotKind::Line,
        PlotKind::Scatter,
        PlotKind::LinePoints,
        PlotKind::Area,
    ];
    for (i, kind) in kinds.iter().enumerate() {
        write_and_check(
            |p| {
                viz::plot_chart(
                    &points,
                    p,
                    800,
                    480,
                    "en",
                    LegendMode::Right,
                    "Test Chart",
                    *kind,
                    0.3,
                    None, // no country styles in tests
                )
                .unwrap();
            },
            &format!("kind{}", i),
        );
    }
}

#[test]
fn legend_modes_produce_files() {
    let points = sample_points();
    let modes = [
        LegendMode::Inside,
        LegendMode::Right,
        LegendMode::Top,
        LegendMode::Bottom,
    ];
    for (i, mode) in modes.iter().enumerate() {
        write_and_check(
            |p| {
                viz::plot_chart(
                    &points,
                    p,
                    800,
                    480,
                    "en",
                    *mode,
                    "Legend Test",
                    PlotKind::LinePoints,
                    0.3,
                    None, // no country styles in tests
                )
                .unwrap();
            },
            &format!("legend{}", i),
        );
    }
}

#[test]
fn empty_points_is_error() {
    let points: Vec<DataPoint> = vec![];
    let tmp = std::env::temp_dir().join("wbd_viz_empty.svg");
    let e = viz::plot_chart(
        &points,
        &tmp,
        800,
        480,
        "en",
        LegendMode::Right,
        "Empty",
        PlotKind::Line,
        0.3,
        None, // no country styles in tests
    );
    assert!(e.is_err());
}
