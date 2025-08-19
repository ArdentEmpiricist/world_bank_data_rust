use world_bank_data_rust::models::{DataPoint, GroupKey};
use world_bank_data_rust::stats::grouped_summary;

fn dp(ind_id: &str, c_iso3: &str, year: i32, v: Option<f64>) -> DataPoint {
    DataPoint {
        indicator_id: ind_id.into(),
        indicator_name: "Dummy".into(),
        country_id: "XX".into(),
        country_name: "Xland".into(),
        country_iso3: c_iso3.into(),
        year,
        value: v,
        unit: None,
        obs_status: None,
        decimal: None,
    }
}

#[test]
fn grouped_stats_handle_missing_and_median_even_odd() {
    // Two groups: (IND1, AAA) with values [1,2,3,4] -> median = (2+3)/2 = 2.5
    //             (IND1, BBB) with [10, None, 30] -> missing = 1, median = 20
    let rows = vec![
        dp("IND1", "AAA", 2018, Some(1.0)),
        dp("IND1", "AAA", 2019, Some(2.0)),
        dp("IND1", "AAA", 2020, Some(3.0)),
        dp("IND1", "AAA", 2021, Some(4.0)),
        dp("IND1", "BBB", 2018, Some(10.0)),
        dp("IND1", "BBB", 2019, None),
        dp("IND1", "BBB", 2020, Some(30.0)),
    ];
    let mut got = grouped_summary(&rows);
    got.sort_by(|a, b| a.key.cmp(&b.key));

    let a = &got[0];
    assert_eq!(
        a.key,
        GroupKey {
            indicator_id: "IND1".into(),
            country_iso3: "AAA".into()
        }
    );
    assert_eq!(a.count, 4);
    assert_eq!(a.missing, 0);
    assert_eq!(a.min, Some(1.0));
    assert_eq!(a.max, Some(4.0));
    assert!((a.mean.unwrap() - 2.5).abs() < 1e-9);
    assert!((a.median.unwrap() - 2.5).abs() < 1e-9);

    let b = &got[1];
    assert_eq!(
        b.key,
        GroupKey {
            indicator_id: "IND1".into(),
            country_iso3: "BBB".into()
        }
    );
    assert_eq!(b.count, 2);
    assert_eq!(b.missing, 1);
    assert_eq!(b.min, Some(10.0));
    assert_eq!(b.max, Some(30.0));
    assert_eq!(b.mean.unwrap(), 20.0);
    assert_eq!(b.median.unwrap(), 20.0);
}
