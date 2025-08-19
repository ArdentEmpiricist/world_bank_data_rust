use world_bank_data_rust::models::{DataPoint, Entry, Meta, IndicatorMeta};

#[test]
fn meta_per_page_accepts_string_or_number() {
    // per_page as string
    let m: Meta =
        serde_json::from_str(r#"{"page":1,"pages":2,"per_page":"1000","total":2000}"#).unwrap();
    assert_eq!(m.per_page, 1000);
    // per_page as number
    let m: Meta =
        serde_json::from_str(r#"{"page":1,"pages":2,"per_page":500,"total":2000}"#).unwrap();
    assert_eq!(m.per_page, 500);
}

#[test]
fn datapoint_from_entry_parses_year_and_names() {
    let e: Entry = serde_json::from_str(
        r#"
    {
      "indicator":{"id":"SP.POP.TOTL","value":"Population, total"},
      "country":{"id":"DE","value":"Germany"},
      "countryiso3code":"DEU",
      "date":"2020",
      "value":83100000,
      "unit":"",
      "obs_status":null,
      "decimal":0
    }"#,
    )
    .unwrap();
    let p = DataPoint::from(e);
    assert_eq!(p.year, 2020);
    assert_eq!(p.indicator_id, "SP.POP.TOTL");
    assert_eq!(p.indicator_name, "Population, total");
    assert_eq!(p.country_iso3, "DEU");
    assert_eq!(p.value, Some(83_100_000.0));
}

#[test]
fn indicator_meta_parses_with_value_alias() {
    // Test parsing with "name" field
    let meta: IndicatorMeta = serde_json::from_str(
        r#"
    {
      "id": "SP.POP.TOTL",
      "name": "Population, total",
      "unit": "Number"
    }"#,
    )
    .unwrap();
    assert_eq!(meta.id, "SP.POP.TOTL");
    assert_eq!(meta.name, "Population, total");
    assert_eq!(meta.unit, Some("Number".to_string()));

    // Test parsing with "value" field (aliased to name)
    let meta: IndicatorMeta = serde_json::from_str(
        r#"
    {
      "id": "NY.GDP.MKTP.CD",
      "value": "GDP (current US$)",
      "unit": "Current US Dollars"
    }"#,
    )
    .unwrap();
    assert_eq!(meta.id, "NY.GDP.MKTP.CD");
    assert_eq!(meta.name, "GDP (current US$)");
    assert_eq!(meta.unit, Some("Current US Dollars".to_string()));
}

#[test]
fn indicator_meta_handles_missing_unit() {
    // Test parsing without unit field
    let meta: IndicatorMeta = serde_json::from_str(
        r#"
    {
      "id": "SP.POP.TOTL",
      "value": "Population, total"
    }"#,
    )
    .unwrap();
    assert_eq!(meta.id, "SP.POP.TOTL");
    assert_eq!(meta.name, "Population, total");
    assert_eq!(meta.unit, None);
}

#[test]
fn fetch_indicator_units_returns_empty_for_empty_input() {
    use world_bank_data_rust::Client;
    let client = Client::default();
    let result = client.fetch_indicator_units(&[]).unwrap();
    assert!(result.is_empty());
}
