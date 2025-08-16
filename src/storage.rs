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