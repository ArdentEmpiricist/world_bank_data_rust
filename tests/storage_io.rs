use std::fs;
use std::path::PathBuf;
use world_bank_data_rust::models::DataPoint;
use world_bank_data_rust::storage;

fn sample(n: usize) -> Vec<DataPoint> {
    (0..n)
        .map(|i| DataPoint {
            indicator_id: "IND".into(),
            indicator_name: "Indicator".into(),
            country_id: "DE".into(),
            country_name: "Germany".into(),
            country_iso3: "DEU".into(),
            year: 2000 + i as i32,
            value: Some(100.0 + i as f64),
            unit: None,
            obs_status: None,
            decimal: None,
        })
        .collect()
}

#[test]
fn save_csv_and_json() {
    let rows = sample(3);
    let tmp = std::env::temp_dir();

    let csv_path: PathBuf = tmp.join("wbd_rs_test.csv");
    storage::save_csv(&rows, &csv_path).unwrap();
    let csv_txt = fs::read_to_string(&csv_path).unwrap();
    assert!(csv_txt.starts_with("indicator_id,indicator_name,"));
    assert_eq!(csv_txt.lines().count(), 1 + rows.len());
    fs::remove_file(&csv_path).ok();

    let json_path: PathBuf = tmp.join("wbd_rs_test.json");
    storage::save_json(&rows, &json_path).unwrap();
    let json_txt = fs::read_to_string(&json_path).unwrap();
    let v: serde_json::Value = serde_json::from_str(&json_txt).unwrap();
    assert!(v.as_array().unwrap().len() == rows.len());
    fs::remove_file(&json_path).ok();
}

//test if the CSV file is save and won't include executable formulas
//this is a security issue, as the CSV file can be opened in Excel and the formulas
//can be executed, which can lead to data loss or other issues
//we prefix the cells with a single quote to avoid this issue
#[test]
fn csv_cells_are_prefixed_to_avoid_formulas() {
    // Arrange: craft a row with classic CSV/Excel injection starters
    let points = vec![DataPoint {
        indicator_id: "=HYPERLINK(\"http://evil\")".into(), // leading '='
        indicator_name: "+SUM(A1:A9)".into(),               // leading '+'
        country_id: "DE".into(),
        country_name: "@foo".into(), // leading '@'
        country_iso3: "DEU".into(),
        year: 2020,
        value: Some(1.0),
        unit: None,
        obs_status: None,
        decimal: None,
    }];

    // Write to a temp CSV using the production function
    let tmp = std::env::temp_dir().join("csv_injection.csv");
    storage::save_csv(&points, &tmp).unwrap();

    // Parse the CSV back and inspect the fields semantically
    let mut rdr = csv::Reader::from_path(&tmp).unwrap();
    let headers = rdr.headers().unwrap().clone();
    let mut rows = rdr.records();
    let row = rows.next().expect("one data row expected").unwrap();

    // Helper to get a cell by header name
    let cell = |name: &str| {
        let idx = headers
            .iter()
            .position(|h| h == name)
            .expect("header present");
        row.get(idx).unwrap()
    };

    // Assert: each risky string field is prefixed with a single quote
    let id = cell("indicator_id");
    assert!(id.starts_with('\''), "indicator_id not prefixed: {id}");
    assert!(
        id.contains("=HYPERLINK"),
        "indicator_id content changed: {id}"
    );

    let name = cell("indicator_name");
    assert!(
        name.starts_with('\''),
        "indicator_name not prefixed: {name}"
    );
    assert!(
        name.contains("+SUM"),
        "indicator_name content changed: {name}"
    );

    let cname = cell("country_name");
    assert!(
        cname.starts_with('\''),
        "country_name not prefixed: {cname}"
    );
    assert!(
        cname.contains("@foo"),
        "country_name content changed: {cname}"
    );

    // Cleanup
    let _ = std::fs::remove_file(tmp);
}
