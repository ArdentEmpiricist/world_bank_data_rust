use std::fs;
use world_bank_data_rust::models::DataPoint;
use world_bank_data_rust::viz::{self, LegendMode, PlotKind};

fn points_three_series() -> Vec<DataPoint> {
    let make = |iso: &str, name: &str, shift: f64| -> Vec<DataPoint> {
        vec![
            DataPoint {
                indicator_id: "X".into(),
                indicator_name: "Demo".into(),
                country_id: iso[..2].into(),
                country_name: name.into(),
                country_iso3: iso.into(),
                year: 2019,
                value: Some(1.0 + shift),
                unit: None,
                obs_status: None,
                decimal: None,
            },
            DataPoint {
                indicator_id: "X".into(),
                indicator_name: "Demo".into(),
                country_id: iso[..2].into(),
                country_name: name.into(),
                country_iso3: iso.into(),
                year: 2020,
                value: Some(2.0 + shift),
                unit: None,
                obs_status: None,
                decimal: None,
            },
            DataPoint {
                indicator_id: "X".into(),
                indicator_name: "Demo".into(),
                country_id: iso[..2].into(),
                country_name: name.into(),
                country_iso3: iso.into(),
                year: 2021,
                value: Some(3.0 + shift),
                unit: None,
                obs_status: None,
                decimal: None,
            },
        ]
    };
    let mut v = Vec::new();
    v.extend(make("DEU", "Germany", 0.0));
    v.extend(make("USA", "United States", 0.5));
    v.extend(make("FRA", "France", 1.0));
    v
}

fn write_and_check(name: &str, f: impl Fn(&std::path::Path)) {
    let path = std::env::temp_dir().join(format!("wbd_viz_extra_{}.svg", name));
    f(&path);
    let meta = fs::metadata(&path).expect("file");
    assert!(meta.len() > 0);
    fs::remove_file(&path).ok();
}

#[test]
fn stacked_area_and_grouped_bar_and_loess() {
    let pts = points_three_series();
    write_and_check("stacked_area", |p| {
        viz::plot_chart(
            &pts,
            p,
            900,
            520,
            "en",
            LegendMode::Top,
            "Stacked",
            PlotKind::StackedArea,
            0.3,
            None, // no country styles in tests
        )
        .unwrap();
    });
    write_and_check("grouped_bar", |p| {
        viz::plot_chart(
            &pts,
            p,
            900,
            520,
            "en",
            LegendMode::Right,
            "Bars",
            PlotKind::GroupedBar,
            0.3,
            None, // no country styles in tests
        )
        .unwrap();
    });
    write_and_check("loess", |p| {
        viz::plot_chart(
            &pts,
            p,
            900,
            520,
            "en",
            LegendMode::Bottom,
            "Loess",
            PlotKind::Loess,
            0.25,
            None, // no country styles in tests
        )
        .unwrap();
    });
}
