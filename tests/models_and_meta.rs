use world_bank_data_rust::models::{Meta, Entry, DataPoint, IndicatorMetadata};

#[test]
fn meta_per_page_accepts_string_or_number() {
    // per_page as string
    let m: Meta = serde_json::from_str(r#"{"page":1,"pages":2,"per_page":"1000","total":2000}"#).unwrap();
    assert_eq!(m.per_page, 1000);
    // per_page as number
    let m: Meta = serde_json::from_str(r#"{"page":1,"pages":2,"per_page":500,"total":2000}"#).unwrap();
    assert_eq!(m.per_page, 500);
}

#[test]
fn datapoint_from_entry_parses_year_and_names() {
    let e: Entry = serde_json::from_str(r#"
    {
      "indicator":{"id":"SP.POP.TOTL","value":"Population, total"},
      "country":{"id":"DE","value":"Germany"},
      "countryiso3code":"DEU",
      "date":"2020",
      "value":83100000,
      "unit":"",
      "obs_status":null,
      "decimal":0
    }"#).unwrap();
    let p = DataPoint::from(e);
    assert_eq!(p.year, 2020);
    assert_eq!(p.indicator_id, "SP.POP.TOTL");
    assert_eq!(p.indicator_name, "Population, total");
    assert_eq!(p.country_iso3, "DEU");
    assert_eq!(p.value, Some(83_100_000.0));
}

#[test]
fn indicator_metadata_parsing() {
    // Test parsing indicator metadata with unit
    let metadata: IndicatorMetadata = serde_json::from_str(r#"
    {
      "id":"NY.GDP.MKTP.CD",
      "value":"GDP (current US$)",
      "unit":"current US$"
    }"#).unwrap();
    assert_eq!(metadata.id, "NY.GDP.MKTP.CD");
    assert_eq!(metadata.value, "GDP (current US$)");
    assert_eq!(metadata.unit, Some("current US$".to_string()));

    // Test parsing indicator metadata without unit
    let metadata_no_unit: IndicatorMetadata = serde_json::from_str(r#"
    {
      "id":"SP.POP.TOTL",
      "value":"Population, total",
      "unit":null
    }"#).unwrap();
    assert_eq!(metadata_no_unit.id, "SP.POP.TOTL");
    assert_eq!(metadata_no_unit.value, "Population, total");
    assert_eq!(metadata_no_unit.unit, None);

    // Test parsing with "name" field (alias for "value")
    let metadata_with_name: IndicatorMetadata = serde_json::from_str(r#"
    {
      "id":"SE.ADT.LITR.ZS",
      "name":"Adult literacy rate, population 15+ years (%)",
      "unit":"% of people ages 15 and above"
    }"#).unwrap();
    assert_eq!(metadata_with_name.id, "SE.ADT.LITR.ZS");
    assert_eq!(metadata_with_name.value, "Adult literacy rate, population 15+ years (%)");
    assert_eq!(metadata_with_name.unit, Some("% of people ages 15 and above".to_string()));
}