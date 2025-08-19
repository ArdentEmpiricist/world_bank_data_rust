/// Persistence helpers for exporting observations as **CSV** or **pretty JSON**.
///
/// Save observations as CSV with a fixed header order.
///
/// The CSV schema matches `models::DataPoint`. Numeric `value` is written as blank if `None`.
///
/// ### Example
/// ```no_run
/// # use wbi_rs::storage;
/// # use wbi_rs::models::DataPoint;
/// let rows: Vec<DataPoint> = vec![];
/// storage::save_csv(&rows, "out.csv")?;
/// # Ok::<(), anyhow::Error>(())
/// ```
///
/// Save observations as pretty JSON array.
///
/// ### Example
/// ```no_run
/// # use wbi_rs::storage;
/// # use wbi_rs::models::DataPoint;
/// let rows: Vec<DataPoint> = vec![];
/// storage::save_json(&rows, "out.json")?;
/// # Ok::<(), anyhow::Error>(())
/// ```
use crate::models::DataPoint;
use anyhow::Result;
use csv::WriterBuilder;
use serde::Serialize;
use std::borrow::Cow;
use std::path::Path;
use tempfile::NamedTempFile;

/// Return a view of `s` that will not be interpreted as a formula by Excel/Calc.
/// Cells beginning with '=', '+', '-', or '@' are prefixed with a single quote.
/// This preserves the exact text while preventing formula execution on open.
fn csv_safe_cell(s: &str) -> Cow<'_, str> {
    match s.as_bytes().first() {
        Some(b'=' | b'+' | b'-' | b'@') => {
            let mut t = String::with_capacity(s.len() + 1);
            t.push('\'');
            t.push_str(s);
            Cow::Owned(t)
        }
        _ => Cow::Borrowed(s),
    }
}

/// Convert `NaN`/`±inf` to `None` so the JSON is always valid and portable.
/// JSON has no representation for non-finite floats; serializing them would error.
fn finite_or_none(x: Option<f64>) -> Option<f64> {
    match x {
        Some(v) if v.is_finite() => Some(v),
        _ => None,
    }
}

/// Write observations to CSV with:
/// - **Deterministic header order**
/// - **Spreadsheet safety** (guard against formula injection)
/// - **Atomic write** (tempfile → rename)
///
/// The `csv` crate handles quoting/escaping and produces RFC-4180 compatible output.
/// Numeric fields are written as numbers; `None` becomes an empty cell.
/// The final rename is atomic on the same filesystem, avoiding partial/corrupt files.
pub fn save_csv<P: AsRef<Path>>(points: &[DataPoint], path: P) -> Result<()> {
    let path = path.as_ref();
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let mut tmp = NamedTempFile::new_in(parent)?;

    {
        let mut wtr = WriterBuilder::new().from_writer(tmp.as_file_mut());

        // Fixed header order for stable downstream processing
        wtr.serialize((
            "indicator_id",
            "indicator_name",
            "country_id",
            "country_name",
            "country_iso3",
            "year",
            "value",
            "unit",
            "obs_status",
            "decimal",
        ))?;

        // Sanitize string-like fields; pass numeric fields as-is
        for p in points {
            let indicator_id = csv_safe_cell(&p.indicator_id);
            let indicator_name = csv_safe_cell(&p.indicator_name);
            let country_id = csv_safe_cell(&p.country_id);
            let country_name = csv_safe_cell(&p.country_name);
            let country_iso3 = csv_safe_cell(&p.country_iso3);

            // Option<String> fields need sanitized owned strings if present
            let unit: Option<String> = p.unit.as_deref().map(|s| csv_safe_cell(s).into_owned());
            let obs_status: Option<String> = p
                .obs_status
                .as_deref()
                .map(|s| csv_safe_cell(s).into_owned());

            wtr.serialize((
                indicator_id.as_ref(),
                indicator_name.as_ref(),
                country_id.as_ref(),
                country_name.as_ref(),
                country_iso3.as_ref(),
                p.year,      // i32
                p.value,     // Option<f64>
                &unit,       // Option<String>
                &obs_status, // Option<String>
                &p.decimal,  // Option<…>
            ))?;
        }

        wtr.flush()?;
    }

    // All bytes are on disk; atomically move the tempfile into place.
    tmp.persist(path)?;
    Ok(())
}

/// Write observations to **pretty-printed JSON** with:
/// - **Atomic write** (tempfile → rename)
/// - **Non-finite number normalization** (`NaN`/`±inf` → `null`)
/// - **Stable field order** (via a dedicated output struct)
///
/// Designed for human readability and downstream ingestion. For maximum compactness,
/// switch `to_writer_pretty` to `to_writer`.
pub fn save_json<P: AsRef<Path>>(points: &[DataPoint], path: P) -> Result<()> {
    let path = path.as_ref();
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let mut tmp = NamedTempFile::new_in(parent)?;

    #[derive(Serialize)]
    struct DataPointOut<'a> {
        indicator_id: &'a str,
        indicator_name: &'a str,
        country_id: &'a str,
        country_name: &'a str,
        country_iso3: &'a str,
        year: i32,
        value: Option<f64>, // normalized to None if non-finite
        unit: Option<&'a str>,
        obs_status: Option<&'a str>,
        decimal: Option<i64>, // normalized to a common integer type
    }

    // Borrowing view keeps memory use modest while guaranteeing a consistent field order.
    let out: Vec<DataPointOut<'_>> = points
        .iter()
        .map(|p| DataPointOut {
            indicator_id: &p.indicator_id,
            indicator_name: &p.indicator_name,
            country_id: &p.country_id,
            country_name: &p.country_name,
            country_iso3: &p.country_iso3,
            year: p.year,
            value: finite_or_none(p.value),
            unit: p.unit.as_deref(),
            obs_status: p.obs_status.as_deref(),
            decimal: p.decimal.map(|d| d as i64),
        })
        .collect();

    {
        let file = tmp.as_file_mut();
        serde_json::to_writer_pretty(file, &out)?;
    }

    // Atomically replace the destination file to avoid partial writes.
    tmp.persist(path)?;
    Ok(())
}
