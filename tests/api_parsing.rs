use wbi_rs::models::{DataPoint, Entry, Meta};

#[test]
fn parse_sample_json() {
    let sample = r#"
    [
      {"page":1,"pages":1,"per_page":"2","total":2},
      [
        {
          "indicator":{"id":"SP.POP.TOTL","value":"Population, total"},
          "country":{"id":"DE","value":"Germany"},
          "countryiso3code":"DEU",
          "date":"2019",
          "value":83000000,
          "unit":"",
          "obs_status":null,
          "decimal":0
        },
        {
          "indicator":{"id":"SP.POP.TOTL","value":"Population, total"},
          "country":{"id":"DE","value":"Germany"},
          "countryiso3code":"DEU",
          "date":"2020",
          "value":83100000,
          "unit":"",
          "obs_status":null,
          "decimal":0
        }
      ]
    ]
    "#;

    let v: serde_json::Value = serde_json::from_str(sample).unwrap();
    let arr = v.as_array().unwrap();
    let meta: Meta = serde_json::from_value(arr[0].clone()).unwrap();
    assert_eq!(meta.page, 1);
    assert_eq!(meta.pages, 1);
    assert_eq!(meta.per_page, 2);
    assert_eq!(meta.total, 2);

    let entries: Vec<Entry> = serde_json::from_value(arr[1].clone()).unwrap();
    assert_eq!(entries.len(), 2);
    let points: Vec<DataPoint> = entries.into_iter().map(DataPoint::from).collect();
    assert_eq!(points[0].country_iso3, "DEU");
    assert_eq!(points[0].year, 2019);
    assert_eq!(points[0].value, Some(83_000_000.0));
}
