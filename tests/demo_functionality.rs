use world_bank_data_rust::models::DataPoint;
use std::collections::BTreeSet;

/// This function replicates the logic from derive_axis_unit to demonstrate
/// the improved unit derivation that prefers DataPoint.unit over indicator name parsing
fn demonstrate_unit_derivation(points: &[DataPoint]) -> Option<String> {
    // First, try to get unit from actual DataPoint.unit field
    let units: BTreeSet<Option<&str>> = points.iter()
        .map(|p| p.unit.as_deref())
        .collect();
    
    // If we have a single non-empty unit across all points, use it
    if units.len() == 1 {
        if let Some(Some(unit)) = units.iter().next() {
            if !unit.trim().is_empty() {
                return Some(unit.to_string());
            }
        }
    }
    
    // Fallback: extract from indicator names if single indicator 
    let names: BTreeSet<&str> = points.iter().map(|p| p.indicator_name.as_str()).collect();
    if names.len() == 1 {
        // This is a simplified version of extract_unit_from_indicator_name
        let name = names.iter().next().unwrap();
        if let Some(open) = name.rfind('(') {
            if let Some(close) = name.rfind(')') {
                if close > open {
                    let inner = name[open + 1..close].trim();
                    if !inner.is_empty() {
                        return Some(inner.to_string());
                    }
                }
            }
        }
    }
    
    None
}

#[test]
fn demonstrate_unit_improvements() {
    println!("\n=== Unit Derivation Improvement Demo ===");
    
    // Scenario 1: DataPoint with proper unit from metadata
    let points_with_metadata_unit = vec![
        DataPoint {
            indicator_id: "NY.GDP.MKTP.CD".to_string(),
            indicator_name: "GDP (current US$)".to_string(),
            country_id: "US".to_string(),
            country_name: "United States".to_string(),
            country_iso3: "USA".to_string(),
            year: 2020,
            value: Some(21_400_000_000_000.0),
            unit: Some("current US$".to_string()), // From metadata!
            obs_status: None,
            decimal: None,
        }
    ];
    
    // Scenario 2: DataPoint without unit (old behavior fallback)
    let points_without_unit = vec![
        DataPoint {
            indicator_id: "NY.GDP.MKTP.CD".to_string(),
            indicator_name: "GDP (current US$)".to_string(),
            country_id: "US".to_string(),
            country_name: "United States".to_string(),
            country_iso3: "USA".to_string(),
            year: 2020,
            value: Some(21_400_000_000_000.0),
            unit: None, // No metadata available
            obs_status: None,
            decimal: None,
        }
    ];
    
    let unit_from_metadata = demonstrate_unit_derivation(&points_with_metadata_unit);
    let unit_from_fallback = demonstrate_unit_derivation(&points_without_unit);
    
    println!("1. With metadata unit: {:?}", unit_from_metadata);
    println!("2. Fallback to parsing: {:?}", unit_from_fallback);
    
    // Both should return "current US$", but the first method is more reliable
    assert_eq!(unit_from_metadata, Some("current US$".to_string()));
    assert_eq!(unit_from_fallback, Some("current US$".to_string()));
    
    println!("✓ Both methods work, but metadata is more reliable!");
    
    // Scenario 3: Show where metadata helps when indicator name parsing fails
    let points_with_complex_name = vec![
        DataPoint {
            indicator_id: "SL.UEM.TOTL.ZS".to_string(),
            indicator_name: "Unemployment rate, total labor force".to_string(), // No parentheses!
            country_id: "US".to_string(),
            country_name: "United States".to_string(),
            country_iso3: "USA".to_string(),
            year: 2020,
            value: Some(8.1),
            unit: Some("% of total labor force".to_string()), // Metadata saves the day!
            obs_status: None,
            decimal: None,
        }
    ];
    
    let unit_complex = demonstrate_unit_derivation(&points_with_complex_name);
    println!("3. Complex name + metadata: {:?}", unit_complex);
    assert_eq!(unit_complex, Some("% of total labor force".to_string()));
    
    println!("✓ Metadata provides units even when indicator name parsing fails!");
    println!("=== Demo Complete ===\n");
}