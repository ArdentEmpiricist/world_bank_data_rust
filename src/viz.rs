use crate::models::DataPoint;
use anyhow::{anyhow, Result};
use plotters::coord::Shift;
use plotters::prelude::*;
use std::path::Path;

/// Generate a simple multi-series line chart from observations.
///
/// Series are grouped by (country_iso3, indicator_id). Multiple indicators/countries
/// can be plotted together.
///
/// Output format is inferred from file extension:
/// - `.svg` → vector SVG
/// - otherwise → PNG bitmap
pub fn plot_lines<P: AsRef<Path>>(
    points: &[DataPoint],
    out_path: P,
    width: u32,
    height: u32,
) -> Result<()> {
    if points.is_empty() {
        return Err(anyhow!("no data to plot"));
    }

    let out_path = out_path.as_ref(); // borrow the Path
    let path_string = out_path.to_string_lossy().into_owned(); // own a String that lives long enough

    let years: Vec<i32> = points.iter().map(|p| p.year).filter(|y| *y != 0).collect();
    let (min_year, max_year) = (
        *years
            .iter()
            .min()
            .ok_or_else(|| anyhow!("no valid years"))?,
        *years
            .iter()
            .max()
            .ok_or_else(|| anyhow!("no valid years"))?,
    );

    let values: Vec<f64> = points.iter().filter_map(|p| p.value).collect();
    if values.is_empty() {
        return Err(anyhow!("no numeric values to plot"));
    }
    let (min_val, max_val) = (
        values.iter().cloned().fold(f64::INFINITY, f64::min),
        values.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
    );

    // Use &str from our owned String; it stays alive for the whole plotting scope.
    if out_path.extension().and_then(|s| s.to_str()) == Some("svg") {
        let root = SVGBackend::new(path_string.as_str(), (width, height)).into_drawing_area();
        draw_chart(root, points, min_year, max_year, min_val, max_val)?;
    } else {
        let root = BitMapBackend::new(path_string.as_str(), (width, height)).into_drawing_area();
        draw_chart(root, points, min_year, max_year, min_val, max_val)?;
    }

    Ok(())
}

/// Helper that draws to any Plotters backend.
/// NOTE: no 'static bounds; we map backend errors into owned `anyhow` errors.
fn draw_chart<DB>(
    root: DrawingArea<DB, Shift>,
    points: &[DataPoint],
    min_year: i32,
    max_year: i32,
    min_val: f64,
    max_val: f64,
) -> Result<()>
where
    DB: DrawingBackend,
{
    root.fill(&WHITE).map_err(|e| anyhow!("{:?}", e))?;

    let mut chart = ChartBuilder::on(&root)
        .margin(20)
        .caption("World Bank Indicator(s)", ("sans-serif", 24))
        .set_label_area_size(LabelAreaPosition::Left, 60)
        .set_label_area_size(LabelAreaPosition::Bottom, 40)
        .build_cartesian_2d(min_year..max_year, min_val..max_val)
        .map_err(|e| anyhow!("{:?}", e))?;

    chart
        .configure_mesh()
        .axis_desc_style(("sans-serif", 16))
        .x_desc("Year")
        .y_desc("Value")
        .draw()
        .map_err(|e| anyhow!("{:?}", e))?;

    use std::collections::BTreeMap;
    let mut groups: BTreeMap<(String, String), Vec<(i32, f64)>> = BTreeMap::new();
    for p in points {
        if let (y, Some(v)) = (p.year, p.value) {
            if y != 0 {
                groups
                    .entry((p.country_iso3.clone(), p.indicator_id.clone()))
                    .or_default()
                    .push((y, v));
            }
        }
    }
    for ((_country, _indicator), series) in groups.iter_mut() {
        series.sort_by_key(|(y, _)| *y);
    }

    for ((country, indicator), series) in &groups {
        chart
            .draw_series(LineSeries::new(
                series.clone(),
                &Palette99::pick((country.len() + indicator.len()) % 99),
            ))
            .map_err(|e| anyhow!("{:?}", e))?
            .label(format!("{} • {}", country, indicator))
            .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &BLACK));
    }

    chart
        .configure_series_labels()
        .border_style(&BLACK)
        .position(SeriesLabelPosition::UpperLeft)
        .background_style(&WHITE.mix(0.8))
        .draw()
        .map_err(|e| anyhow!("{:?}", e))?;

    root.present().map_err(|e| anyhow!("{:?}", e))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::DataPoint;
    use tempfile::tempdir;

    #[test]
    fn plot_png_and_svg() {
        let dir = tempdir().unwrap();
        let png = dir.path().join("c.png");
        let svg = dir.path().join("c.svg");
        let pts = vec![
            DataPoint {
                indicator_id: "SP.POP.TOTL".into(),
                indicator_name: "Population".into(),
                country_id: "DE".into(),
                country_name: "Germany".into(),
                country_iso3: "DEU".into(),
                year: 2019,
                value: Some(83_000_000.0),
                unit: None,
                obs_status: None,
                decimal: None,
            },
            DataPoint {
                indicator_id: "SP.POP.TOTL".into(),
                indicator_name: "Population".into(),
                country_id: "DE".into(),
                country_name: "Germany".into(),
                country_iso3: "DEU".into(),
                year: 2020,
                value: Some(83_100_000.0),
                unit: None,
                obs_status: None,
                decimal: None,
            },
        ];
        plot_lines(&pts, &png, 600, 400).unwrap();
        plot_lines(&pts, &svg, 600, 400).unwrap();
        assert!(png.exists());
        assert!(svg.exists());
    }
}
