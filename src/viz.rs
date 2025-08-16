use crate::models::DataPoint;
use anyhow::{Result, anyhow};
use num_format::{Locale, ToFormattedString};
use plotters::coord::Shift;
use plotters::prelude::*;
use plotters::style::FontStyle;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

/// Legend placement options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegendMode {
    Inside,
    Right,
    Top,
}

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

/// Generate a simple multi-series line chart from observations (default locale = "en", legend = Right).
pub fn plot_lines<P: AsRef<Path>>(
    points: &[DataPoint],
    out_path: P,
    width: u32,
    height: u32,
) -> Result<()> {
    plot_lines_locale_with_legend(points, out_path, width, height, "en", LegendMode::Right)
}

/// Same as `plot_lines` but with a locale tag for label formatting (e.g., "en" or "de").
pub fn plot_lines_locale<P: AsRef<Path>>(
    points: &[DataPoint],
    out_path: P,
    width: u32,
    height: u32,
    locale_tag: &str,
) -> Result<()> {
    plot_lines_locale_with_legend(
        points,
        out_path,
        width,
        height,
        locale_tag,
        LegendMode::Right,
    )
}

/// Fully configurable plotting: choose locale and legend placement.
pub fn plot_lines_locale_with_legend<P: AsRef<Path>>(
    points: &[DataPoint],
    out_path: P,
    width: u32,
    height: u32,
    locale_tag: &str,
    legend: LegendMode,
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
            root, points, min_year, max_year, min_val, max_val, num_locale, legend,
        )?;
    } else {
        let root = BitMapBackend::new(path_string.as_str(), (width, height)).into_drawing_area();
        draw_chart(
            root, points, min_year, max_year, min_val, max_val, num_locale, legend,
        )?;
    }

    Ok(())
}

// --- helper to draw a separate legend panel ---
fn draw_legend_panel<DB: DrawingBackend>(
    legend_area: &DrawingArea<DB, Shift>,
    items: &[(String, RGBAColor)],
    title: &str,
) -> anyhow::Result<()> {
    legend_area
        .fill(&WHITE)
        .map_err(|e| anyhow::anyhow!("{:?}", e))?;

    // Draw the title of the legend
    let title_font = ("sans-serif", 16).into_font().style(FontStyle::Bold);
    legend_area
        .draw(&Text::new(title, (8, 20), title_font))
        .map_err(|e| anyhow::anyhow!("{:?}", e))?;

    // Draw the legend items
    let (_, h) = legend_area.dim_in_pixel();
    let mut y = 44i32;
    let step = 22i32;

    // Iterate over items and draw each swatch + label
    for (label, color) in items.iter() {
        if y as u32 + 8 >= h {
            break;
        }
        let swatch = PathElement::new(vec![(8, y), (32, y)], color.clone());
        legend_area
            .draw(&swatch)
            .map_err(|e| anyhow::anyhow!("{:?}", e))?;
        // Draw the label next to the swatch
        legend_area
            .draw(&Text::new(label.as_str(), (38, y + 4), ("sans-serif", 14)))
            .map_err(|e| anyhow::anyhow!("{:?}", e))?;

        y += step;
    }

    Ok(())
}

/// Draw chart with chosen legend placement.
fn draw_chart<DB>(
    root: DrawingArea<DB, Shift>,
    points: &[DataPoint],
    min_year: i32,
    max_year: i32,
    min_val: f64,
    max_val: f64,
    num_locale: &Locale,
    legend: LegendMode,
) -> anyhow::Result<()>
where
    DB: DrawingBackend,
{
    // Decide layout based on legend mode
    let (plot_area, legend_area_opt): (DrawingArea<DB, Shift>, Option<DrawingArea<DB, Shift>>) =
        match legend {
            LegendMode::Right => {
                let (plot, legend) = root.split_horizontally((85).percent_width());
                (plot, Some(legend))
            }
            LegendMode::Top => {
                let (legend, plot) = root.split_vertically(64); // ~64 px for legend row
                (plot, Some(legend))
            }
            LegendMode::Inside => (root, None),
        };

    plot_area
        .fill(&WHITE)
        .map_err(|e| anyhow::anyhow!("{:?}", e))?;
    if let Some(ref legend_area) = legend_area_opt {
        legend_area
            .fill(&WHITE)
            .map_err(|e| anyhow::anyhow!("{:?}", e))?;
    }

    // Build chart on the plotting area
    let mut chart = ChartBuilder::on(&plot_area)
        .margin(16)
        .caption("World Bank Indicator(s)", ("sans-serif", 24))
        // Make room so the vertical y-axis title "Value" never overlaps tick labels
        .set_label_area_size(LabelAreaPosition::Left, 100)
        .set_label_area_size(LabelAreaPosition::Bottom, 56)
        .build_cartesian_2d(min_year..max_year, min_val..max_val)
        .map_err(|e| anyhow::anyhow!("{:?}", e))?;

    let y_label_fmt = |v: &f64| {
        let n = (*v).round() as i64;
        n.to_formatted_string(num_locale)
    };
    let x_label_fmt = |y: &i32| y.to_string();

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
        .label_style(("sans-serif", 12)) // was 14
        .axis_desc_style(("sans-serif", 16))
        .draw()
        .map_err(|e| anyhow::anyhow!("{:?}", e))?;

    // Map IDs to human-readable names from the API
    let mut indicator_name_by_id: HashMap<String, String> = HashMap::new();
    let mut country_name_by_iso3: HashMap<String, String> = HashMap::new();
    for p in points {
        indicator_name_by_id
            .entry(p.indicator_id.clone())
            .or_insert_with(|| p.indicator_name.clone());
        country_name_by_iso3
            .entry(p.country_iso3.clone())
            .or_insert_with(|| p.country_name.clone());
    }

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

    let mut legend_items: Vec<(String, RGBAColor)> = Vec::new();
    let inside_mode = matches!(legend, LegendMode::Inside);

    for (idx, ((country_iso3, indicator_id), series)) in groups.iter().enumerate() {
        let color = Palette99::pick(idx).to_rgba();
        let style = ShapeStyle {
            color: color.clone(),
            filled: false,
            stroke_width: 2,
        };

        // Resolve full display names
        let country_label = country_name_by_iso3
            .get(country_iso3)
            .cloned()
            .unwrap_or_else(|| country_iso3.clone());
        let indicator_label = indicator_name_by_id
            .get(indicator_id)
            .cloned()
            .unwrap_or_else(|| indicator_id.clone());
        let legend_label = format!("{} \u{2014} {}", country_label, indicator_label); // em-dash looks tidy

        let mut elem = chart
            .draw_series(LineSeries::new(series.clone(), style))
            .map_err(|e| anyhow::anyhow!("{:?}", e))?;

        if inside_mode {
            // Keep the overlay legend variant consistent with the external one
            let legend_color = color.clone();
            elem = elem.label(legend_label.clone()).legend(move |(x, y)| {
                PathElement::new(vec![(x, y), (x + 24, y)], legend_color.clone())
            });
        } else {
            // External legend panel
            legend_items.push((legend_label, color.clone()));
        }
    }

    if inside_mode {
        chart
            .configure_series_labels()
            .border_style(&BLACK)
            .position(SeriesLabelPosition::UpperLeft)
            .background_style(&WHITE.mix(0.85))
            .label_font(("sans-serif", 14))
            .draw()
            .map_err(|e| anyhow::anyhow!("{:?}", e))?;
    } else if let Some(ref legend_area) = legend_area_opt {
        // Render external legend: in the right panel or top band
        let title = match legend {
            LegendMode::Right => "Legend",
            LegendMode::Top => "Legend",
            LegendMode::Inside => unreachable!(),
        };
        let area_ref = if matches!(legend, LegendMode::Top) {
            // When legend is on top, we created (legend, plot); we need the legend area as-is.
            &legend_area
        } else {
            &legend_area
        };
        draw_legend_panel(area_ref, &legend_items, title)?;
    }

    plot_area
        .present()
        .map_err(|e| anyhow::anyhow!("{:?}", e))?;
    if let Some(legend_area) = legend_area_opt {
        legend_area
            .present()
            .map_err(|e| anyhow::anyhow!("{:?}", e))?;
    }
    Ok(())
}
