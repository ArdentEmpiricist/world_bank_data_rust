use crate::models::DataPoint;
use anyhow::{anyhow, Result};
use num_format::{Locale, ToFormattedString};
use plotters::coord::Shift;
use plotters::prelude::*;
use std::path::Path;

/// Map a user-provided locale tag to a num-format Locale and decimal separator.
/// Supported tags (case-insensitive): "en", "us", "en_US", "de", "de_DE", "german", "fr", "es", "it", "pt", "nl"
fn map_locale(tag: &str) -> (&'static Locale, char) {
    match tag.to_lowercase().as_str() {
        "de" | "de_de" | "german" => (&Locale::de, ','),
        "fr" | "fr_fr" => (&Locale::fr, ','),
        "es" | "es_es" => (&Locale::es, ','),
        "it" | "it_it" => (&Locale::it, ','),
        "pt" | "pt_pt" | "pt_br" => (&Locale::pt, ','),
        "nl" | "nl_nl" => (&Locale::nl, ','),
        _ => (&Locale::en, '.'),
    }
}

/// Generate a simple multi-series line chart from observations (default locale = "en").
pub fn plot_lines<P: AsRef<Path>>(
    points: &[DataPoint],
    out_path: P,
    width: u32,
    height: u32,
) -> Result<()> {
    plot_lines_locale(points, out_path, width, height, "en")
}

/// Same as `plot_lines` but with a locale tag for label formatting (e.g., "en" or "de").
pub fn plot_lines_locale<P: AsRef<Path>>(
    points: &[DataPoint],
    out_path: P,
    width: u32,
    height: u32,
    locale_tag: &str,
) -> Result<()> {
    if points.is_empty() {
        return Err(anyhow!("no data to plot"));
    }

    let out_path = out_path.as_ref();
    let path_string = out_path.to_string_lossy().into_owned();

    let years: Vec<i32> = points.iter().map(|p| p.year).filter(|y| *y != 0).collect();
    let (mut min_year, mut max_year) = (
        *years
            .iter()
            .min()
            .ok_or_else(|| anyhow!("no valid years"))?,
        *years
            .iter()
            .max()
            .ok_or_else(|| anyhow!("no valid years"))?,
    );
    if min_year == max_year {
        min_year -= 1;
        max_year += 1;
    }

    let values: Vec<f64> = points.iter().filter_map(|p| p.value).collect();
    if values.is_empty() {
        return Err(anyhow!("no numeric values to plot"));
    }
    let (mut min_val, mut max_val) = (
        values.iter().cloned().fold(f64::INFINITY, f64::min),
        values.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
    );
    if (max_val - min_val).abs() < std::f64::EPSILON {
        min_val -= 1.0;
        max_val += 1.0;
    }

    let (num_locale, _dec_sep) = map_locale(locale_tag);

    if out_path.extension().and_then(|s| s.to_str()) == Some("svg") {
        let root = SVGBackend::new(path_string.as_str(), (width, height)).into_drawing_area();
        draw_chart(
            root, points, min_year, max_year, min_val, max_val, num_locale,
        )?;
    } else {
        let root = BitMapBackend::new(path_string.as_str(), (width, height)).into_drawing_area();
        draw_chart(
            root, points, min_year, max_year, min_val, max_val, num_locale,
        )?;
    }

    Ok(())
}

/// Helper that draws to any Plotters backend.
fn draw_chart<DB>(
    root: DrawingArea<DB, Shift>,
    points: &[DataPoint],
    min_year: i32,
    max_year: i32,
    min_val: f64,
    max_val: f64,
    num_locale: &Locale,
) -> Result<()>
where
    DB: DrawingBackend,
{
    root.fill(&WHITE).map_err(|e| anyhow!("{:?}", e))?;

    let mut chart = ChartBuilder::on(&root)
        .margin(20)
        .caption("World Bank Indicator(s)", ("sans-serif", 24))
        .set_label_area_size(LabelAreaPosition::Left, 80)
        .set_label_area_size(LabelAreaPosition::Bottom, 44)
        .build_cartesian_2d(min_year..max_year, min_val..max_val)
        .map_err(|e| anyhow!("{:?}", e))?;

    // Axis label formatters: Y uses locale thousands separators; integers only
    let y_label_fmt = |v: &f64| {
        let n = (*v).round() as i64;
        n.to_formatted_string(num_locale)
    };
    let x_label_fmt = |y: &i32| y.to_string();

    // Limit label counts to avoid overlap
    let x_label_count = ((max_year - min_year + 1) as usize).min(12);
    let y_label_count = 10usize;

    chart
        .configure_mesh()
        .x_desc("Year")
        .y_desc("Value")
        .x_labels(x_label_count)
        .y_labels(y_label_count)
        .x_label_formatter(&x_label_fmt)
        .y_label_formatter(&y_label_fmt)
        .label_style(("sans-serif", 14))
        .axis_desc_style(("sans-serif", 16))
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

    // Distinct color per series, thicker strokes
    for (idx, ((country, indicator), series)) in groups.iter().enumerate() {
        // Base palette color -> RGBA (so we can reuse it in style & legend)
        let color = Palette99::pick(idx).to_rgba();

        // Use the same color for the line stroke
        let style = ShapeStyle {
            color: color.clone(),
            filled: false,
            stroke_width: 2,
        };

        chart
            .draw_series(LineSeries::new(series.clone(), style))
            .map_err(|e| anyhow!("{:?}", e))?
            .label(format!("{} • {}", country, indicator))
            // Move the color into the closure; clone for each legend glyph draw
            .legend(move |(x, y)| PathElement::new(vec![(x, y), (x + 24, y)], color.clone()));
    }

    chart
        .configure_series_labels()
        .border_style(&BLACK)
        .position(SeriesLabelPosition::UpperLeft)
        .background_style(&WHITE.mix(0.85))
        .label_font(("sans-serif", 14))
        .draw()
        .map_err(|e| anyhow!("{:?}", e))?;

    root.present().map_err(|e| anyhow!("{:?}", e))?;
    Ok(())
}
