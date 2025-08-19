use world_bank_data_rust::models::{DataPoint, Entry, Meta};

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
