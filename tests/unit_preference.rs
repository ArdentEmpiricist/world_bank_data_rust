use world_bank_data_rust::models::DataPoint;
use world_bank_data_rust::viz::util::derive_axis_unit;

fn make_data_point(
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
fn derive_axis_unit_prefers_consistent_api_unit() {
    // Case 1: All points have the same API-provided unit
    let points = vec![
        make_data_point(
            "IND1",
            "Test Indicator",
            "DEU",
            2019,
            Some(100.0),
            Some("Number"),
        ),
        make_data_point(
            "IND1",
            "Test Indicator",
            "DEU",
            2020,
            Some(200.0),
            Some("Number"),
        ),
        make_data_point(
            "IND1",
            "Test Indicator",
            "USA",
            2019,
            Some(300.0),
            Some("Number"),
        ),
    ];

    assert_eq!(derive_axis_unit(&points), Some("Number".to_string()));
}

#[test]
fn derive_axis_unit_fallback_to_indicator_name() {
    // Case 2: No consistent API unit, fallback to single indicator name parsing
    let points = vec![
        make_data_point("IND1", "GDP (current US$)", "DEU", 2019, Some(100.0), None),
        make_data_point(
            "IND1",
            "GDP (current US$)",
            "DEU",
            2020,
            Some(200.0),
            Some(""),
        ), // empty unit
        make_data_point("IND1", "GDP (current US$)", "USA", 2019, Some(300.0), None),
    ];

    assert_eq!(derive_axis_unit(&points), Some("current US$".to_string()));
}

#[test]
fn derive_axis_unit_mixed_api_units_fallback() {
    // Case 3: Different API units, fallback to single indicator name parsing
    let points = vec![
        make_data_point(
            "IND1",
            "Test (annual %)",
            "DEU",
            2019,
            Some(1.0),
            Some("Percent"),
        ),
        make_data_point(
            "IND1",
            "Test (annual %)",
            "DEU",
            2020,
            Some(2.0),
            Some("Number"),
        ), // different unit
    ];

    assert_eq!(derive_axis_unit(&points), Some("annual %".to_string()));
}

#[test]
fn derive_axis_unit_multiple_indicators_no_unit() {
    // Case 4: Multiple indicators, no consistent unit
    let points = vec![
        make_data_point("IND1", "GDP (current US$)", "DEU", 2019, Some(100.0), None),
        make_data_point("IND2", "Population", "DEU", 2019, Some(1000000.0), None),
    ];

    assert_eq!(derive_axis_unit(&points), None);
}

#[test]
fn derive_axis_unit_empty_and_none_units_ignored() {
    // Case 5: Mix of None and empty string units should be ignored
    let points = vec![
        make_data_point("IND1", "Test", "DEU", 2019, Some(1.0), None),
        make_data_point("IND1", "Test", "DEU", 2020, Some(2.0), Some("")),
        make_data_point("IND1", "Test", "DEU", 2021, Some(3.0), Some("USD")),
    ];

    assert_eq!(derive_axis_unit(&points), Some("USD".to_string()));
}
