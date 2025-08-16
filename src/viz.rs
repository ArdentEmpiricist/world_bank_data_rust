//! Visualization utilities for world_bank_data_rust
//!
//! This module renders tidy World Bank observations into **SVG** or **PNG** line charts.
//!
//! Highlights:
//! - Distinct color per series (Palette99)
//! - Locale-aware Y-axis labels (e.g., `30,000` vs `30.000`), whole numbers only
//! - Legend placement: `Inside` (overlay), `Right` (panel), `Top` (band), `Bottom` (band)
//! - Non-overlapping external legends with truncation/wrapping
//! - Custom chart title via the title-aware entry point
//!
//! Typical usage (library):
//! ```no_run
//! # use world_bank_data_rust::viz::{self, LegendMode};
//! # use world_bank_data_rust::{Client, DateSpec};
//! # fn main() -> anyhow::Result<()> {
//! let client = Client::default();
//! let rows = client.fetch(&["DEU".into()], &["SP.POP.TOTL".into()], Some(DateSpec::Range{ start: 2010, end: 2020 }), None)?;
//! viz::plot_lines_locale_with_legend_title(&rows, "pop.svg", 1000, 600, "de", LegendMode::Top, "Population, total (2010–2020)")?;
//! # Ok(()) }
//! ```

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
    /// Overlay legend inside the plotting area (may overlap data).
    Inside,
    /// Separate, non-overlapping legend panel on the right side.
    Right,
    /// Separate, non-overlapping legend band at the top.
    Top,
    /// Separate, non-overlapping legend band at the bottom.
    Bottom,
}

/// Map a user-provided locale tag to a `num_format::Locale` and its decimal separator char.
///
/// Supported tags (case-insensitive): `en`, `us`, `en_US`, `de`, `de_DE`, `german`,
/// `fr`, `es`, `it`, `pt`, `nl`. Defaults to English.
fn map_locale(tag: &str) -> (&'static Locale, char) {
    match tag.to_lowercase().as_str() {
        "de" | "de_de" | "german" => (&Locale::de, ','),
        "fr" | "fr_fr" => (&Locale::fr, ','),
        "es" | "es_es" => (&Locale::es, ','),
        "it" | "it_it" => (&Locale::it, ','),
        "pt" | "pt_pt" | "pt_br" => (&Locale::pt, ','),
        "nl" | "nl_nl" => (&Locale::nl, ','),
        _ => (&Locale::en, '.'), // default
    }
}

// Microsoft Office (2013+) chart series palette.
// Order: Blue, Orange, Gray, Gold, Light Blue, Green, Dark Blue, Dark Orange, Dark Gray, Brownish Gold.
const OFFICE10: [RGBColor; 10] = [
    RGBColor(68, 114, 196),  // blue      (#4472C4)
    RGBColor(237, 125, 49),  // orange    (#ED7D31)
    RGBColor(165, 165, 165), // gray      (#A5A5A5)
    RGBColor(255, 192, 0),   // gold      (#FFC000)
    RGBColor(91, 155, 213),  // light blue(#5B9BD5)
    RGBColor(112, 173, 71),  // green     (#70AD47)
    RGBColor(38, 68, 120),   // dark blue (#264478)
    RGBColor(158, 72, 14),   // dark org. (#9E480E)
    RGBColor(99, 99, 99),    // dark gray (#636363)
    RGBColor(153, 115, 0),   // brownish  (#997300)
];

#[inline]
fn office_color(idx: usize) -> RGBAColor {
    OFFICE10[idx % OFFICE10.len()].to_rgba()
}

/// Convenience: plot with default locale (`"en"`) and default legend (`Right`).
pub fn plot_lines<P: AsRef<Path>>(
    points: &[DataPoint],
    out_path: P,
    width: u32,
    height: u32,
) -> Result<()> {
    plot_lines_locale_with_legend_title(
        points,
        out_path,
        width,
        height,
        "en",
        LegendMode::Right,
        "World Bank Indicator(s)",
    )
}

/// Convenience: plot with chosen locale and default legend (`Right`).
pub fn plot_lines_locale<P: AsRef<Path>>(
    points: &[DataPoint],
    out_path: P,
    width: u32,
    height: u32,
    locale_tag: &str,
) -> Result<()> {
    plot_lines_locale_with_legend_title(
        points,
        out_path,
        width,
        height,
        locale_tag,
        LegendMode::Right,
        "World Bank Indicator(s)",
    )
}

/// Convenience: plot with chosen locale and legend (default title).
pub fn plot_lines_locale_with_legend<P: AsRef<Path>>(
    points: &[DataPoint],
    out_path: P,
    width: u32,
    height: u32,
    locale_tag: &str,
    legend: LegendMode,
) -> Result<()> {
    plot_lines_locale_with_legend_title(
        points,
        out_path,
        width,
        height,
        locale_tag,
        legend,
        "World Bank Indicator(s)",
    )
}

/// Fully-configurable entry point: choose locale, legend placement, and custom chart title.
pub fn plot_lines_locale_with_legend_title<P: AsRef<Path>>(
    points: &[DataPoint],
    out_path: P,
    width: u32,
    height: u32,
    locale_tag: &str,
    legend: LegendMode,
    title: &str,
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
        // Expand a flat X-domain slightly to keep Plotters happy
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
        // Expand a flat Y-range so ticks render nicely
        min_val -= 1.0;
        max_val += 1.0;
    }

    let (num_locale, _dec_sep) = map_locale(locale_tag);

    if out_path.extension().and_then(|s| s.to_str()) == Some("svg") {
        let root = SVGBackend::new(path_string.as_str(), (width, height)).into_drawing_area();
        draw_chart(
            root, points, min_year, max_year, min_val, max_val, num_locale, legend, title,
        )?;
    } else {
        let root = BitMapBackend::new(path_string.as_str(), (width, height)).into_drawing_area();
        draw_chart(
            root, points, min_year, max_year, min_val, max_val, num_locale, legend, title,
        )?;
    }

    Ok(())
}

/// Heuristic: estimate pixel width of text (Plotters has no built-in text measuring).
fn estimate_text_width_px(text: &str, font_px: u32) -> u32 {
    // ~0.58–0.62 * font_px per glyph is a decent heuristic; use 0.60.
    ((text.chars().count() as f32) * (font_px as f32) * 0.60).ceil() as u32
}

/// Truncate to fit `max_px` and add a single ellipsis if needed.
fn truncate_to_width(text: &str, font_px: u32, max_px: u32) -> String {
    let mut out = String::new();
    for ch in text.chars() {
        let next = format!("{out}{ch}");
        if estimate_text_width_px(&next, font_px) > max_px {
            if !out.is_empty() {
                if estimate_text_width_px(&(out.clone() + "…"), font_px) <= max_px {
                    out.push('…');
                } else if out.len() > 1 {
                    out.pop();
                    out.push('…');
                }
            }
            return out;
        }
        out = next;
    }
    out
}

/// Draw a legend either on the right (single column), top (flow), or bottom (flow).
/// - Uses a centered **circle** marker aligned with the text baseline.
/// - Truncates labels to avoid spilling outside the legend area.
fn draw_legend_panel<DB: DrawingBackend>(
    legend_area: &DrawingArea<DB, Shift>,
    items: &[(String, RGBAColor)],
    title: &str,
    placement: LegendMode,
) -> anyhow::Result<()> {
    legend_area
        .fill(&WHITE)
        .map_err(|e| anyhow::anyhow!("{:?}", e))?;

    // Title (bold)
    let title_font = ("sans-serif", 16).into_font().style(FontStyle::Bold);
    legend_area
        .draw(&Text::new(title, (8, 20), title_font))
        .map_err(|e| anyhow::anyhow!("{:?}", e))?;

    let (w, _) = legend_area.dim_in_pixel();
    let pad_x: i32 = 8;
    let start_y: i32 = 44;
    let font_px: u32 = 14;
    let row_h: i32 = 22;

    match placement {
        LegendMode::Right => {
            // Single column; each line truncated to panel width.
            let mut y = start_y;
            for (label, color) in items {
                let max_text_w = w.saturating_sub(48); // 24 marker+gap + ~24 padding
                let text = truncate_to_width(label, font_px, max_text_w);
                // Center the color marker with the text baseline (same y)
                let marker_x = pad_x + 12;
                let text_x = pad_x + 24;
                legend_area
                    .draw(&Circle::new((marker_x, y), 4, color.clone().filled()))
                    .map_err(|e| anyhow::anyhow!("{:?}", e))?;
                legend_area
                    .draw(&Text::new(
                        text.as_str(),
                        (text_x, y),
                        ("sans-serif", font_px),
                    ))
                    .map_err(|e| anyhow::anyhow!("{:?}", e))?;
                y += row_h;
            }
        }
        LegendMode::Top | LegendMode::Bottom => {
            // Flow layout across full width, wrapping as needed.
            let mut x = pad_x;
            let mut y = start_y;
            for (label, color) in items {
                let max_item_w = (w as i32 - x - pad_x).max(40) as u32;
                // Reserve ~28 px for marker + gap; the rest for text
                let text_max = max_item_w.saturating_sub(28);
                let text = truncate_to_width(label, font_px, text_max);

                let text_w = estimate_text_width_px(&text, font_px) as i32;
                let item_w = (28 + text_w + 12).min(w as i32 - 2 * pad_x); // include some right gap

                // Wrap if this item would overflow the row
                if x + item_w > (w as i32 - pad_x) {
                    x = pad_x;
                    y += row_h;
                }

                let marker_x = x + 12;
                let text_x = x + 24;

                legend_area
                    .draw(&Circle::new((marker_x, y), 4, color.clone().filled()))
                    .map_err(|e| anyhow::anyhow!("{:?}", e))?;
                legend_area
                    .draw(&Text::new(
                        text.as_str(),
                        (text_x, y),
                        ("sans-serif", font_px),
                    ))
                    .map_err(|e| anyhow::anyhow!("{:?}", e))?;

                x += item_w;
            }
        }
        LegendMode::Inside => unreachable!("legend panel is not used for Inside mode"),
    }

    Ok(())
}

/// Draw chart with chosen legend placement & custom title.
fn draw_chart<DB>(
    root: DrawingArea<DB, Shift>,
    points: &[DataPoint],
    min_year: i32,
    max_year: i32,
    min_val: f64,
    max_val: f64,
    num_locale: &Locale,
    legend: LegendMode,
    title: &str,
) -> anyhow::Result<()>
where
    DB: DrawingBackend,
{
    // Decide layout based on legend mode.
    // For external legends we split the drawing area so the legend never overlaps data.
    let (plot_area, legend_area_opt): (DrawingArea<DB, Shift>, Option<DrawingArea<DB, Shift>>) =
        match legend {
            LegendMode::Right => {
                let (plot, legend) = root.split_horizontally((85).percent_width());
                (plot, Some(legend))
            }
            LegendMode::Top => {
                let (legend, plot) = root.split_vertically(64); // ~64 px for legend band
                (plot, Some(legend))
            }
            LegendMode::Bottom => {
                let (plot, legend) = root.split_vertically((85).percent_height()); // 15% height for legend
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

    // Build chart on the plotting area.
    // Larger label areas prevent the "Value" caption from overlapping tick labels.
    let mut chart = ChartBuilder::on(&plot_area)
        .margin(16)
        .caption(title, ("sans-serif", 24))
        .set_label_area_size(LabelAreaPosition::Left, 100)
        .set_label_area_size(LabelAreaPosition::Bottom, 56)
        .build_cartesian_2d(min_year..max_year, min_val..max_val)
        .map_err(|e| anyhow::anyhow!("{:?}", e))?;

    // Axis label formatters: integers with locale thousands separators
    let y_label_fmt = |v: &f64| {
        let n = (*v).round() as i64;
        n.to_formatted_string(num_locale)
    };
    let x_label_fmt = |y: &i32| y.to_string();

    // Limit label counts to reduce collisions
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
        .label_style(("sans-serif", 12)) // slightly smaller to avoid overlaps
        .axis_desc_style(("sans-serif", 16))
        .draw()
        .map_err(|e| anyhow::anyhow!("{:?}", e))?;

    // Build name lookups for long legend labels
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

    // Group as (ISO3, indicator_id) -> Vec<(year, value)>
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

    // Draw series and collect legend items
    let mut legend_items: Vec<(String, RGBAColor)> = Vec::new();
    let inside_mode = matches!(legend, LegendMode::Inside);

    for (idx, ((country_iso3, indicator_id), series)) in groups.iter().enumerate() {
        //let color = Palette99::pick(idx).to_rgba();
        let color = office_color(idx);
        let style = ShapeStyle {
            color: color.clone(),
            filled: false,
            stroke_width: 2,
        };

        // Resolve long label: "{Country Name} — {Indicator Name}"
        let country_label = country_name_by_iso3
            .get(country_iso3)
            .cloned()
            .unwrap_or_else(|| country_iso3.clone());
        let indicator_label = indicator_name_by_id
            .get(indicator_id)
            .cloned()
            .unwrap_or_else(|| indicator_id.clone());
        let legend_label = format!("{} — {}", country_label, indicator_label);

        let mut series_elem = chart
            .draw_series(LineSeries::new(series.clone(), style))
            .map_err(|e| anyhow::anyhow!("{:?}", e))?;

        if inside_mode {
            // Compose an *inside* legend entry using a centered marker + text.
            let legend_color = color.clone();
            let legend_text = legend_label.clone();
            series_elem = series_elem
                .label(legend_text.clone())
                .legend(move |(x, y)| {
                    EmptyElement::at((x, y))
                        + Circle::new((x + 8, y), 4, legend_color.clone().filled())
                        + Text::new(legend_text.clone(), (x + 20, y), ("sans-serif", 14))
                });
        } else {
            // External panel: collect items to render later.
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
        draw_legend_panel(legend_area, &legend_items, "Legend", legend)?;
    }

    plot_area
        .present()
        .map_err(|e| anyhow::anyhow!("{:?}", e))?;
    if let Some(ref legend_area) = legend_area_opt {
        legend_area
            .present()
            .map_err(|e| anyhow::anyhow!("{:?}", e))?;
    }
    Ok(())
}
