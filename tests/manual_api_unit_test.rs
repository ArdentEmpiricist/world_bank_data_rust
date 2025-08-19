use wbi_rs::{models::DataPoint, viz};

#[test]
fn test_api_unit_in_chart_output() {
    // Create test data with API-provided units
    let points = vec![
        DataPoint {
            indicator_id: "SP.POP.TOTL".into(),
            indicator_name: "Population, total".into(),
            country_id: "DE".into(),
            country_name: "Germany".into(),
            country_iso3: "DEU".into(),
            year: 2019,
            value: Some(83_000_000.0),
            unit: Some("Number".into()), // API-provided unit
            obs_status: None,
            decimal: None,
        },
        DataPoint {
            indicator_id: "SP.POP.TOTL".into(),
            indicator_name: "Population, total".into(),
            country_id: "DE".into(),
            country_name: "Germany".into(),
            country_iso3: "DEU".into(),
            year: 2020,
            value: Some(83_100_000.0),
            unit: Some("Number".into()), // API-provided unit
            obs_status: None,
            decimal: None,
        },
    ];

    // Generate a chart
    let output_path = std::env::temp_dir().join("test_api_unit_chart.svg");
    viz::plot_chart(
        &points,
        &output_path,
        800,
        480,
        "en",
        viz::LegendMode::Right,
        "Test API Unit Chart",
        viz::PlotKind::LinePoints,
        0.3,
        None, // no country styles in tests
    )
    .unwrap();

    // Check if the SVG contains the API-provided unit
    let svg_content = std::fs::read_to_string(&output_path).unwrap();

    // The Y-axis title should contain "Number" from the API-provided unit
    // It may also have scaling like "Number (millions)"
    assert!(
        svg_content.to_lowercase().contains("number"),
        "Expected Y-axis to contain API-provided unit 'Number'. SVG content: {}",
        svg_content
    );

    // Clean up
    std::fs::remove_file(&output_path).ok();
}
