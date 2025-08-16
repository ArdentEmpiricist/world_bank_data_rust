use crate::models::{DataPoint, GroupKey};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Summary statistics for a group.
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
pub fn grouped_summary(points: &[DataPoint]) -> Vec<Summary> {
    let mut groups: BTreeMap<GroupKey, Vec<f64>> = BTreeMap::new();
    let mut missing: BTreeMap<GroupKey, usize> = BTreeMap::new();
    for p in points {
        let key = GroupKey {
            indicator_id: p.indicator_id.clone(),
            country_iso3: p.country_iso3.clone(),
        };
        match p.value {
            Some(v) => groups.entry(key).or_default().push(v),
            None => *missing.entry(key).or_default() += 1,
        }
    }

    let mut out = Vec::new();
    for (key, mut vals) in groups {
        vals.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let count = vals.len();
        let min = vals.first().cloned();
        let max = vals.last().cloned();
        let mean = if count > 0 {
            Some(vals.iter().copied().sum::<f64>() / count as f64)
        } else { None };
        let median = if count == 0 {
            None
        } else if count % 2 == 1 {
            Some(vals[count / 2])
        } else {
            Some((vals[count / 2 - 1] + vals[count / 2]) / 2.0)
        };
        let miss = missing.get(&key).cloned().unwrap_or(0);
        out.push(Summary { key, count, missing: miss, min, max, mean, median });
    }
    out
}