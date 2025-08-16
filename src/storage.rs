use crate::models::DataPoint;
use anyhow::Result;
use csv::WriterBuilder;
use std::fs::File;
use std::io::Write;
use std::path::Path;

/// Save observations as CSV with header.
pub fn save_csv<P: AsRef<Path>>(points: &[DataPoint], path: P) -> Result<()> {
    let mut wtr = WriterBuilder::new().from_path(path)?;
    wtr.serialize(("indicator_id","indicator_name","country_id","country_name","country_iso3","year","value","unit","obs_status","decimal"))?;
    for p in points {
        wtr.serialize((
            &p.indicator_id,
            &p.indicator_name,
            &p.country_id,
            &p.country_name,
            &p.country_iso3,
            p.year,
            p.value,
            &p.unit,
            &p.obs_status,
            &p.decimal,
        ))?;
    }
    wtr.flush()?;
    Ok(())
}

/// Save observations as pretty JSON array.
pub fn save_json<P: AsRef<Path>>(points: &[DataPoint], path: P) -> Result<()> {
    let mut f = File::create(path)?;
    let s = serde_json::to_string_pretty(points)?;
    f.write_all(s.as_bytes())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use crate::models::DataPoint;

    #[test]
    fn write_csv_and_json() {
        let dir = tempdir().unwrap();
        let csvp = dir.path().join("x.csv");
        let jsonp = dir.path().join("x.json");
        let pts = vec![DataPoint {
            indicator_id: "A".into(),
            indicator_name: "Alpha".into(),
            country_id: "DE".into(),
            country_name: "Germany".into(),
            country_iso3: "DEU".into(),
            year: 2000,
            value: Some(1.23),
            unit: None, obs_status: None, decimal: None,
        }];
        save_csv(&pts, &csvp).unwrap();
        save_json(&pts, &jsonp).unwrap();
        assert!(csvp.exists());
        assert!(jsonp.exists());
    }
}