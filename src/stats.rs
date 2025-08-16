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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::DataPoint;

    #[test]
    fn summary_basic() {
        let key = GroupKey { indicator_id: "X".into(), country_iso3: "DEU".into() };
        let mut pts = Vec::new();
        for v in [1.0, 2.0, 3.0, 4.0] {
            pts.push(DataPoint {
                indicator_id: "X".into(),
                indicator_name: "X Name".into(),
                country_id: "DE".into(),
                country_name: "Germany".into(),
                country_iso3: "DEU".into(),
                year: 2000,
                value: Some(v),
                unit: None, obs_status: None, decimal: None
            });
        }
        // add one missing
        pts.push(DataPoint {
            indicator_id: "X".into(),
            indicator_name: "X Name".into(),
            country_id: "DE".into(),
            country_name: "Germany".into(),
            country_iso3: "DEU".into(),
            year: 2001,
            value: None, unit: None, obs_status: None, decimal: None
        });

        let s = grouped_summary(&pts);
        assert_eq!(s.len(), 1);
        let s0 = &s[0];
        assert_eq!(s0.key, key);
        assert_eq!(s0.count, 4);
        assert_eq!(s0.missing, 1);
        assert_eq!(s0.min, Some(1.0));
        assert_eq!(s0.max, Some(4.0));
        assert_eq!(s0.mean, Some(2.5));
        assert_eq!(s0.median, Some((2.0+3.0)/2.0));
    }
}