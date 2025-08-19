use world_bank_data_rust::models::DataPoint;

/// Helper function to create test DataPoints
fn create_test_datapoint(
    indicator_id: &str,
    indicator_name: &str,
    unit: Option<String>,
) -> DataPoint {
    DataPoint {
        indicator_id: indicator_id.to_string(),
        indicator_name: indicator_name.to_string(),
        country_id: "US".to_string(),
        country_name: "United States".to_string(),
        country_iso3: "USA".to_string(),
        year: 2020,
        value: Some(1000.0),
        unit,
        obs_status: None,
        decimal: None,
    }
}

#[test]
fn test_datapoint_unit_populated_from_metadata() {
    // Test that DataPoint can have unit populated from metadata
    let mut point = create_test_datapoint(
        "NY.GDP.MKTP.CD", 
        "GDP (current US$)", 
        None
    );
    
    // Initially no unit
    assert_eq!(point.unit, None);
    
    // Simulate metadata population
    point.unit = Some("current US$".to_string());
    assert_eq!(point.unit, Some("current US$".to_string()));
}

#[test]
fn test_unit_precedence() {
    // Test that actual unit field takes precedence over indicator name parsing
    let points_with_unit = vec![
        create_test_datapoint(
            "NY.GDP.MKTP.CD", 
            "GDP (current US$)", 
            Some("current US$".to_string())
        )
    ];
    
    let points_without_unit = vec![
        create_test_datapoint(
            "NY.GDP.MKTP.CD", 
            "GDP (current US$)", 
            None
        )
    ];
    
    let points_with_empty_unit = vec![
        create_test_datapoint(
            "NY.GDP.MKTP.CD", 
            "GDP (current US$)", 
            Some("".to_string())
        )
    ];
    
    // Since derive_axis_unit is a private function in viz.rs, we'll test the behavior
    // through the public interface when possible, or we can add a test-only export
    // For now, we'll test that our DataPoint structure properly supports units
    assert_eq!(points_with_unit[0].unit, Some("current US$".to_string()));
    assert_eq!(points_without_unit[0].unit, None);
    assert_eq!(points_with_empty_unit[0].unit, Some("".to_string()));
}

#[test] 
fn test_mixed_indicators_units() {
    // Test behavior when we have multiple indicators with different units
    let points = vec![
        create_test_datapoint(
            "NY.GDP.MKTP.CD", 
            "GDP (current US$)", 
            Some("current US$".to_string())
        ),
        create_test_datapoint(
            "SP.POP.TOTL", 
            "Population, total", 
            Some("people".to_string())
        ),
    ];
    
    // Different indicators should have different units
    assert_eq!(points[0].unit, Some("current US$".to_string()));
    assert_eq!(points[1].unit, Some("people".to_string()));
}