use world_bank_data_rust::models::DataPoint;
use world_bank_data_rust::storage;
use std::fs;
use std::path::PathBuf;

fn sample(n: usize) -> Vec<DataPoint> {
    (0..n).map(|i| DataPoint {
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
    }).collect()
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