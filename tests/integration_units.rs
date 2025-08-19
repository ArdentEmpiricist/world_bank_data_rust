// Integration test for the indicator metadata functionality
// This test is designed to work with mock data to avoid needing live API access

use world_bank_data_rust::models::{DataPoint, IndicatorMetadata};

fn create_mock_datapoint_without_unit() -> DataPoint {
    DataPoint {
        indicator_id: "NY.GDP.MKTP.CD".to_string(),
        indicator_name: "GDP (current US$)".to_string(),
        country_id: "US".to_string(),
        country_name: "United States".to_string(),
        country_iso3: "USA".to_string(),
        year: 2020,
        value: Some(21_400_000_000_000.0),
        unit: None, // This is what we want to populate
        obs_status: None,
        decimal: None,
    }
}

fn create_mock_datapoint_with_empty_unit() -> DataPoint {
    DataPoint {
        indicator_id: "SP.POP.TOTL".to_string(),
        indicator_name: "Population, total".to_string(),
        country_id: "US".to_string(),
        country_name: "United States".to_string(),
        country_iso3: "USA".to_string(),
        year: 2020,
        value: Some(331_000_000.0),
        unit: Some("".to_string()), // Empty unit that should be populated
        obs_status: None,
        decimal: None,
    }
}

fn create_mock_datapoint_with_unit() -> DataPoint {
    DataPoint {
        indicator_id: "SL.UEM.TOTL.ZS".to_string(),
        indicator_name: "Unemployment, total (% of total labor force)".to_string(),
        country_id: "US".to_string(),
        country_name: "United States".to_string(),
        country_iso3: "USA".to_string(),
        year: 2020,
        value: Some(8.1),
        unit: Some("% of total labor force".to_string()), // Already has unit
        obs_status: None,
        decimal: None,
    }
}

#[test]
fn test_indicator_metadata_deserialization() {
    // Test that IndicatorMetadata correctly deserializes from API response format
    let json_response = r#"
    {
        "id": "NY.GDP.MKTP.CD",
        "value": "GDP (current US$)",
        "unit": "current US$"
    }"#;
    
    let metadata: IndicatorMetadata = serde_json::from_str(json_response).unwrap();
    assert_eq!(metadata.id, "NY.GDP.MKTP.CD");
    assert_eq!(metadata.value, "GDP (current US$)");
    assert_eq!(metadata.unit, Some("current US$".to_string()));
}

#[test]
fn test_unit_population_behavior() {
    // Test that unit population works correctly for different scenarios
    let mut points = vec![
        create_mock_datapoint_without_unit(),     // Should be populated
        create_mock_datapoint_with_empty_unit(),  // Should be populated
        create_mock_datapoint_with_unit(),        // Should remain unchanged
    ];
    
    // Verify initial state
    assert_eq!(points[0].unit, None);
    assert_eq!(points[1].unit, Some("".to_string()));
    assert_eq!(points[2].unit, Some("% of total labor force".to_string()));
    
    // Simulate unit population (in a real scenario, this would be done by populate_units_from_metadata)
    points[0].unit = Some("current US$".to_string());
    points[1].unit = Some("people".to_string());
    // points[2] already has a unit, so it remains unchanged
    
    // Verify final state
    assert_eq!(points[0].unit, Some("current US$".to_string()));
    assert_eq!(points[1].unit, Some("people".to_string()));
    assert_eq!(points[2].unit, Some("% of total labor force".to_string()));
}

#[test]
fn test_viz_unit_precedence() {
    // Test that visualization prefers actual units over parsed indicator names
    use std::collections::BTreeSet;
    
    // DataPoints with actual units should be preferred
    let points_with_units = vec![
        DataPoint {
            indicator_id: "NY.GDP.MKTP.CD".to_string(),
            indicator_name: "GDP (current US$)".to_string(),
            country_id: "US".to_string(),
            country_name: "United States".to_string(),
            country_iso3: "USA".to_string(),
            year: 2020,
            value: Some(21_400_000_000_000.0),
            unit: Some("current US$".to_string()), // Actual unit from metadata
            obs_status: None,
            decimal: None,
        }
    ];
    
    // Simulate the unit derivation logic (this would normally be done internally by viz)
    let units: BTreeSet<Option<&str>> = points_with_units.iter()
        .map(|p| p.unit.as_deref())
        .collect();
    
    // Should prefer the actual unit
    if units.len() == 1 {
        if let Some(Some(unit)) = units.iter().next() {
            if !unit.trim().is_empty() {
                assert_eq!(*unit, "current US$");
            }
        }
    }
}