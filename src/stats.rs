use crate::models::{DataPoint, GroupKey};
use serde::{Deserialize, Serialize};

/// Simple grouped summary statistics.
///
/// Summary statistics per `(indicator_id, country_iso3)` group.
#[doc = "- `count`: number of non-missing values"]
#[doc = "- `missing`: number of missing values"]
#[doc = "- `min`/`max`: extremes over non-missing"]
#[doc = "- `mean`: arithmetic mean"]
#[doc = "- `median`: middle value (average of two middles for even length)"]
///
/// Compute grouped statistics by `(indicator_id, country_iso3)`.
///
/// - Missing values are **not** included in `count`, but are reflected in `missing`.
/// - Sorting is stable and based on the tuple key, so results are deterministic.
///
/// ### Example
/// ```
/// use world_bank_data_rust::models::DataPoint;
/// use world_bank_data_rust::stats::grouped_summary;
///
/// let rows = vec![
///     DataPoint { indicator_id: "X".into(), indicator_name: "Demo".into(),
///                 country_id:"DE".into(), country_name:"Germany".into(), country_iso3:"DEU".into(),
///                 year: 2020, value: Some(1.0), unit: None, obs_status: None, decimal: None },
///     DataPoint { indicator_id: "X".into(), indicator_name: "Demo".into(),
///                 country_id:"DE".into(), country_name:"Germany".into(), country_iso3:"DEU".into(),
///                 year: 2021, value: None, unit: None, obs_status: None, decimal: None },
/// ];
/// let s = grouped_summary(&rows);
/// assert_eq!(s[0].count, 1);
/// assert_eq!(s[0].missing, 1);
/// ```

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Summary {
    pub key: GroupKey,
    pub count: usize,
    pub missing: usize,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub mean: Option<f64>,
    pub median: Option<f64>,
}

/// Compute grouped statistics by (indicator_id, country_iso3).
/// This function aggregates `DataPoint` entries into summaries.
/// With finite-value guard + safe sort
pub fn grouped_summary(points: &[DataPoint]) -> Vec<Summary> {
    use std::cmp::Ordering;
    use std::collections::BTreeMap;

    let mut groups: BTreeMap<GroupKey, Vec<f64>> = BTreeMap::new();
    let mut missing: BTreeMap<GroupKey, usize> = BTreeMap::new();

    for p in points {
        let key = GroupKey {
            indicator_id: p.indicator_id.clone(),
            country_iso3: p.country_iso3.clone(),
        };

        match p.value {
            // Treat only finite numbers as valid observations
            Some(v) if v.is_finite() => {
                groups.entry(key).or_default().push(v);
            }
            // Count None or non-finite values as "missing"
            _ => {
                *missing.entry(key).or_default() += 1;
            }
        }
    }

    let mut out = Vec::new();

    for (key, mut vals) in groups {
        // Safe float sort (no unwrap panic even if weird floats slipped through)
        vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));

        let count = vals.len();
        let min = vals.first().cloned();
        let max = vals.last().cloned();

        let mean = if count > 0 {
            Some(vals.iter().copied().sum::<f64>() / count as f64)
        } else {
            None
        };

        let median = if count == 0 {
            None
        } else if count % 2 == 1 {
            Some(vals[count / 2])
        } else {
            Some((vals[count / 2 - 1] + vals[count / 2]) / 2.0)
        };

        let miss = missing.get(&key).cloned().unwrap_or(0);

        out.push(Summary {
            key,
            count,
            missing: miss,
            min,
            max,
            mean,
            median,
        });
    }

    out
}
