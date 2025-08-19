//! Tests for grouped statistics across multiple indicators and countries.

use wbi_rs::{models::DataPoint, stats};

fn create_multi_indicator_multi_country_data() -> Vec<DataPoint> {
    vec![
        // USA GDP data
        DataPoint {
            country_iso3: "USA".to_string(),
            country_name: "United States".to_string(),
            country_id: "US".to_string(),
            indicator_id: "NY.GDP.MKTP.CD".to_string(),
            indicator_name: "GDP (current US$)".to_string(),
            year: 2020,
            value: Some(20_950_000_000_000.0),
            unit: Some("current US$".to_string()),
            obs_status: None,
            decimal: None,
        },
        DataPoint {
            country_iso3: "USA".to_string(),
            country_name: "United States".to_string(),
            country_id: "US".to_string(),
            indicator_id: "NY.GDP.MKTP.CD".to_string(),
            indicator_name: "GDP (current US$)".to_string(),
            year: 2021,
            value: Some(23_320_000_000_000.0),
            unit: Some("current US$".to_string()),
            obs_status: None,
            decimal: None,
        },
        // USA Population data
        DataPoint {
            country_iso3: "USA".to_string(),
            country_name: "United States".to_string(),
            country_id: "US".to_string(),
            indicator_id: "SP.POP.TOTL".to_string(),
            indicator_name: "Population, total".to_string(),
            year: 2020,
            value: Some(331_002_651.0),
            unit: Some("people".to_string()),
            obs_status: None,
            decimal: None,
        },
        DataPoint {
            country_iso3: "USA".to_string(),
            country_name: "United States".to_string(),
            country_id: "US".to_string(),
            indicator_id: "SP.POP.TOTL".to_string(),
            indicator_name: "Population, total".to_string(),
            year: 2021,
            value: Some(332_031_554.0),
            unit: Some("people".to_string()),
            obs_status: None,
            decimal: None,
        },
        // DEU GDP data
        DataPoint {
            country_iso3: "DEU".to_string(),
            country_name: "Germany".to_string(),
            country_id: "DE".to_string(),
            indicator_id: "NY.GDP.MKTP.CD".to_string(),
            indicator_name: "GDP (current US$)".to_string(),
            year: 2020,
            value: Some(3_846_000_000_000.0),
            unit: Some("current US$".to_string()),
            obs_status: None,
            decimal: None,
        },
        DataPoint {
            country_iso3: "DEU".to_string(),
            country_name: "Germany".to_string(),
            country_id: "DE".to_string(),
            indicator_id: "NY.GDP.MKTP.CD".to_string(),
            indicator_name: "GDP (current US$)".to_string(),
            year: 2021,
            value: Some(4_260_000_000_000.0),
            unit: Some("current US$".to_string()),
            obs_status: None,
            decimal: None,
        },
        // DEU Population data
        DataPoint {
            country_iso3: "DEU".to_string(),
            country_name: "Germany".to_string(),
            country_id: "DE".to_string(),
            indicator_id: "SP.POP.TOTL".to_string(),
            indicator_name: "Population, total".to_string(),
            year: 2020,
            value: Some(83_240_525.0),
            unit: Some("people".to_string()),
            obs_status: None,
            decimal: None,
        },
        DataPoint {
            country_iso3: "DEU".to_string(),
            country_name: "Germany".to_string(),
            country_id: "DE".to_string(),
            indicator_id: "SP.POP.TOTL".to_string(),
            indicator_name: "Population, total".to_string(),
            year: 2021,
            value: Some(83_196_078.0),
            unit: Some("people".to_string()),
            obs_status: None,
            decimal: None,
        },
    ]
}

#[test]
fn test_grouped_summary_multi_indicator_multi_country() {
    let data = create_multi_indicator_multi_country_data();
    let summaries = stats::grouped_summary(&data);

    // Should have 4 groups: USA+GDP, USA+Pop, DEU+GDP, DEU+Pop
    assert_eq!(summaries.len(), 4);

    // Check that all expected combinations are present
    let mut found_groups = std::collections::HashSet::new();
    for summary in &summaries {
        let group_key = format!("{}+{}", summary.key.country_iso3, summary.key.indicator_id);
        found_groups.insert(group_key);
        
        // Each group should have exactly 2 data points (2020 and 2021)
        assert_eq!(summary.count, 2);
        
        // All values should be valid
        assert!(summary.mean.is_some());
        assert!(summary.median.is_some());
        assert!(summary.min.is_some());
        assert!(summary.max.is_some());
    }

    // Verify we have all expected group combinations
    assert!(found_groups.contains("USA+NY.GDP.MKTP.CD"));
    assert!(found_groups.contains("USA+SP.POP.TOTL"));
    assert!(found_groups.contains("DEU+NY.GDP.MKTP.CD"));
    assert!(found_groups.contains("DEU+SP.POP.TOTL"));
}

#[test]
fn test_grouped_summary_preserves_group_structure() {
    let data = create_multi_indicator_multi_country_data();
    let summaries = stats::grouped_summary(&data);

    for summary in summaries {
        // Check that the keys are correctly structured
        assert!(summary.key.indicator_id == "NY.GDP.MKTP.CD" || summary.key.indicator_id == "SP.POP.TOTL");
        assert!(summary.key.country_iso3 == "USA" || summary.key.country_iso3 == "DEU");
        
        // Each group should have count=2, missing=0 (all test data has values)
        assert_eq!(summary.count, 2);
        assert_eq!(summary.missing, 0);
    }
}