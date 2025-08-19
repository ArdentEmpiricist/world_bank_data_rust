use wbi_rs::models::DataPoint;

/// Helper to create a DataPoint for testing
fn make_test_datapoint(
    indicator_id: &str,
    indicator_name: &str,
    country_iso3: &str,
    year: i32,
    value: Option<f64>,
    unit: Option<&str>,
) -> DataPoint {
    DataPoint {
        indicator_id: indicator_id.into(),
        indicator_name: indicator_name.into(),
        country_id: "XX".into(),
        country_name: "Test Country".into(),
        country_iso3: country_iso3.into(),
        year,
        value,
        unit: unit.map(|s| s.into()),
        obs_status: None,
        decimal: None,
    }
}

#[test]
fn test_datapoint_unit_enrichment_concept() {
    // This test verifies the concept of unit enrichment that the fetch method implements

    // Simulate DataPoints without units (as they might come from the API)
    let mut points = vec![
        make_test_datapoint(
            "SP.POP.TOTL",
            "Population, total",
            "DEU",
            2019,
            Some(83000000.0),
            None,
        ),
        make_test_datapoint(
            "SP.POP.TOTL",
            "Population, total",
            "DEU",
            2020,
            Some(83200000.0),
            Some(""),
        ), // empty unit
        make_test_datapoint(
            "NY.GDP.MKTP.CD",
            "GDP (current US$)",
            "DEU",
            2019,
            Some(3.86e12),
            None,
        ),
    ];

    // Simulate indicator metadata that would be fetched
    let mut indicator_units = std::collections::HashMap::new();
    indicator_units.insert("SP.POP.TOTL".to_string(), "Number".to_string());
    indicator_units.insert(
        "NY.GDP.MKTP.CD".to_string(),
        "Current US Dollars".to_string(),
    );

    // Simulate the enrichment logic from the fetch method
    for point in &mut points {
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

    // Verify enrichment worked
    assert_eq!(points[0].unit, Some("Number".to_string()));
    assert_eq!(points[1].unit, Some("Number".to_string())); // empty string was enriched
    assert_eq!(points[2].unit, Some("Current US Dollars".to_string()));
}

#[test]
fn test_unit_enrichment_preserves_existing_units() {
    // Test that existing non-empty units are preserved
    let mut points = vec![
        make_test_datapoint(
            "SP.POP.TOTL",
            "Population, total",
            "DEU",
            2019,
            Some(83000000.0),
            Some("People"),
        ),
        make_test_datapoint(
            "SP.POP.TOTL",
            "Population, total",
            "DEU",
            2020,
            Some(83200000.0),
            None,
        ),
    ];

    let mut indicator_units = std::collections::HashMap::new();
    indicator_units.insert("SP.POP.TOTL".to_string(), "Number".to_string());

    // Simulate enrichment logic
    for point in &mut points {
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

    // First point should keep its original unit, second should be enriched
    assert_eq!(points[0].unit, Some("People".to_string())); // preserved
    assert_eq!(points[1].unit, Some("Number".to_string())); // enriched
}
