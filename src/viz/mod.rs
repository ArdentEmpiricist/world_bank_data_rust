//! Visualization utilities: render multi-series charts to **SVG** or **PNG**.
//!
//! - Distinct series colors (Microsoft Office palette)
//! - Locale-aware tick labels (`30,000` vs `30.000`), whole numbers
//! - Legend placement: `Inside`, `Right`, `Top`, `Bottom` (non-overlapping for external legends)
//! - Plot kinds: `Line`, `Scatter`, `LinePoints`, `Area`, `StackedArea`, `GroupedBar`, `Loess`
//! - Custom chart title and legend handling for long labels

pub mod legend;
pub mod loess;
pub mod text;
pub mod types;
pub mod util;

// Re-export types for public API
pub use types::{DEFAULT_LEGEND_MODE, LegendMode, PlotKind};

// Re-export style modules (transitional)
pub use crate::viz_style as style;

use crate::models::DataPoint;
use anyhow::{Result, anyhow};
use num_format::Locale;

use plotters::backend::DrawingBackend;
use plotters::coord::Shift;
use plotters::prelude::*;
use plotters::series::{AreaSeries, LineSeries};

use plotters::style::FontFamily;

use plotters_bitmap::BitMapBackend;
use plotters_svg::SVGBackend;

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;
use std::sync::Once;

use legend::{draw_legend_panel, estimate_top_bottom_legend_height_px};
use util::{
    choose_axis_scale, compute_left_label_area_px, derive_axis_unit, is_percentage_like,
    map_locale, office_color,
};



use loess::loess_series;

/// One-time registration for a fallback "sans-serif" font when using the `ab_glyph` text path.
/// Required because `ab_glyph` doesn't discover OS fonts.
static INIT_FONTS: Once = Once::new();

fn ensure_fonts_registered() {
    // Safe to call many times; only runs once.
    INIT_FONTS.call_once(|| {
        // Updated path for new module location: from `src/viz/mod.rs` → project root → `assets/DejaVuSans.ttf`
        let _ = plotters::style::register_font(
            "sans-serif",
            plotters::style::FontStyle::Normal,
            include_bytes!("../../assets/DejaVuSans.ttf"),
        );
    });
}

/// Convenience: plot with default locale (`"en"`) and default legend (`Bottom`) as a line chart.
pub fn plot_lines<P: AsRef<Path>>(
    points: &[DataPoint],
    out_path: P,
    width: u32,
    height: u32,
) -> Result<()> {
    plot_chart(
        points,
        out_path,
        width,
        height,
        "en",
        DEFAULT_LEGEND_MODE,
        "World Bank Indicator(s)",
        PlotKind::Line,
        0.3, // default LOESS span
        None, // no country styles
    )
}

/// Convenience: plot with chosen locale and default legend (`Bottom`) as a line chart.
pub fn plot_lines_locale<P: AsRef<Path>>(
    points: &[DataPoint],
    out_path: P,
    width: u32,
    height: u32,
    locale_tag: &str,
) -> Result<()> {
    plot_chart(
        points,
        out_path,
        width,
        height,
        locale_tag,
        DEFAULT_LEGEND_MODE,
        "World Bank Indicator(s)",
        PlotKind::Line,
        0.3,
        None, // no country styles
    )
}

/// Convenience: plot with chosen locale and legend (default title) as a line chart.
pub fn plot_lines_locale_with_legend<P: AsRef<Path>>(
    points: &[DataPoint],
    out_path: P,
    width: u32,
    height: u32,
    locale_tag: &str,
    legend: LegendMode,
) -> Result<()> {
    plot_chart(
        points,
        out_path,
        width,
        height,
        locale_tag,
        legend,
        "World Bank Indicator(s)",
        PlotKind::Line,
        0.3,
        None, // no country styles
    )
}

/// Convenience: plot with chosen locale, legend, and custom title as a line chart.
pub fn plot_lines_locale_with_legend_title<P: AsRef<Path>>(
    points: &[DataPoint],
    out_path: P,
    width: u32,
    height: u32,
    locale_tag: &str,
    legend: LegendMode,
    title: &str,
) -> Result<()> {
    plot_chart(
        points,
        out_path,
        width,
        height,
        locale_tag,
        legend,
        title,
        PlotKind::Line,
        0.3,
        None, // no country styles
    )
}

/// Fully-configurable entry point: choose locale, legend placement, custom title, plot kind, and LOESS span.
#[allow(clippy::too_many_arguments)]
pub fn plot_chart<P: AsRef<Path>>(
    points: &[DataPoint],
    out_path: P,
    width: u32,
    height: u32,
    locale_tag: &str,
    legend: LegendMode,
    title: &str,
    kind: PlotKind,
    loess_span: f64, // fraction of neighbors (0,1], used only for PlotKind::Loess
    country_styles: Option<bool>, // None when feature disabled, Some(bool) when enabled
) -> Result<()> {
    if points.is_empty() {
        return Err(anyhow!("no data to plot"));
    }
    ensure_fonts_registered();
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
    if (max_val - min_val).abs() < f64::EPSILON {
        min_val -= 1.0;
        max_val += 1.0;
    }

    let (num_locale, _dec_sep) = map_locale(locale_tag);

    if out_path.extension().and_then(|s| s.to_str()) == Some("svg") {
        let root = SVGBackend::new(path_string.as_str(), (width, height)).into_drawing_area();
        draw_chart(
            root, points, min_year, max_year, min_val, max_val, num_locale, legend, title, kind,
            loess_span, country_styles,
        )?;
    } else {
        let root = BitMapBackend::new(path_string.as_str(), (width, height)).into_drawing_area();
        draw_chart(
            root, points, min_year, max_year, min_val, max_val, num_locale, legend, title, kind,
            loess_span, country_styles,
        )?;
    }
    Ok(())
}

// This is the main chart drawing function - copied from original viz.rs
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn draw_chart<DB>(
    root: DrawingArea<DB, Shift>,
    points: &[DataPoint],
    min_year: i32,
    max_year: i32,
    min_val: f64,
    max_val: f64,
    _num_locale: &Locale,
    legend: LegendMode,
    title: &str,
    kind: PlotKind,
    loess_span: f64,
    country_styles: Option<bool>,
) -> Result<()>
where
    DB: DrawingBackend,
{
    // ----------------------------
    // 0) Common constants
    // ----------------------------
    const MARGIN: i32 = 16; // matches .margin(16) below
    let x_min = min_year as f64;
    let x_max = max_year as f64;

    // Axis scaling for large magnitudes (thousands/millions/billions/…)
    // Derive a unit from the indicator metadata/name, then decide scaling.
    // Percent-like units are NOT scaled; currencies/counts can be scaled to thousands/millions/…
    let base_unit = derive_axis_unit(points); // e.g., "current US$" or "annual %"
    let max_abs = min_val.abs().max(max_val.abs());

    let (yscale, scale_word) = if let Some(ref unit) = base_unit {
        if is_percentage_like(unit) {
            (1.0, "") // do not scale percentages
        } else {
            choose_axis_scale(max_abs) // e.g., (1e6, "millions")
        }
    } else {
        // Mixed indicators or no unit => fall back to generic scaling
        choose_axis_scale(max_abs)
    };

    // This is the final Y-axis title
    let y_axis_title = match (base_unit.as_deref(), scale_word) {
        (Some(u), "") => u.to_string(),         // e.g., "annual %"
        (Some(u), sw) => format!("{u} ({sw})"), // e.g., "current US$ (millions)"
        (None, "") => "Value".to_string(),
        (None, sw) => format!("Value ({sw})"),
    };

    // X/Y tick formatters
    let x_label_fmt = |x: &f64| (x.round() as i32).to_string();
    let y_label_fmt_scaled = |v: &f64| {
        let a = v.abs();
        let prec = if a >= 100.0 {
            0
        } else if a >= 10.0 {
            1
        } else {
            2
        };
        format!("{:.*}", prec, *v)
    };
    let x_label_count = ((max_year - min_year + 1) as usize).min(12);
    let y_label_count = 10usize;

    // ----------------------------
    // 1) Build name maps & groups
    // ----------------------------
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
        if let (y, Some(v)) = (p.year, p.value)
            && y != 0
        {
            groups
                .entry((p.country_iso3.clone(), p.indicator_id.clone()))
                .or_default()
                .push((y, v));
        }
    }
    for ((_country, _indicator), series) in groups.iter_mut() {
        series.sort_by_key(|(y, _)| *y);
    }

    // Sorted list by *country name* then *indicator name*
    let mut series_list: Vec<(String, String, String, String, Vec<(i32, f64)>)> = Vec::new();
    for ((iso3, indicator_id), series) in groups.iter() {
        let country_label = country_name_by_iso3
            .get(iso3)
            .cloned()
            .unwrap_or_else(|| iso3.clone());
        let indicator_label = indicator_name_by_id
            .get(indicator_id)
            .cloned()
            .unwrap_or_else(|| indicator_id.clone());
        series_list.push((
            iso3.clone(),
            indicator_id.clone(),
            country_label,
            indicator_label,
            series.clone(),
        ));
    }
    series_list.sort_by(|a, b| a.2.cmp(&b.2).then(a.3.cmp(&b.3)));

    // Shorter legend labels when possible:
    // - one indicator across many countries → label = country name only
    // - one country across many indicators → label = indicator name only
    // - both vary → "Country — Indicator"
    let unique_indicators: BTreeSet<&str> =
        points.iter().map(|p| p.indicator_id.as_str()).collect();
    let unique_countries: BTreeSet<&str> = points.iter().map(|p| p.country_iso3.as_str()).collect();
    let one_indicator = unique_indicators.len() == 1;
    let one_country = unique_countries.len() == 1;

    let make_label = |country_label: &str, indicator_label: &str| -> String {
        if one_indicator && !one_country {
            country_label.to_string()
        } else if one_country && !one_indicator {
            indicator_label.to_string()
        } else {
            format!("{} — {}", country_label, indicator_label)
        }
    };

    // ----------------------------
    // 2) Compute dynamic gutters before splitting
    // ----------------------------
    // Left label area depends on *scaled* Y range & tick font size (12)
    let left_label_width_px =
        compute_left_label_area_px(min_val / yscale, max_val / yscale, y_label_count, 12);
    // X-axis text column starts at margin + left label area
    let axis_x_start_px: i32 = MARGIN + left_label_width_px as i32;

    // Legend height for Top/Bottom: pre-measure how much vertical space we need.
    // Build the list of final legend texts in drawing order (matches series_list).
    let legend_texts: Vec<String> = series_list
        .iter()
        .map(|(_iso3, _ind, country_label, indicator_label, _s)| {
            make_label(country_label, indicator_label)
        })
        .collect();

    let (root_w_u32, root_h_u32) = root.dim_in_pixel();
    let root_w = root_w_u32 as i32;
    let root_h = root_h_u32 as i32;

    // Title is generally omitted (best practice). We pass "" later.
    let _has_title = false;
    let _title_font_px: u32 = 16;
    let _font_px: u32 = 14;

    // Estimator to avoid missing-symbol issues:
    let legend_needed_h = if matches!(legend, LegendMode::Top | LegendMode::Bottom) {
        estimate_top_bottom_legend_height_px(
            &legend_texts,
            axis_x_start_px,
            root_w,
            /* has_title: */ false, // we render without a legend title by default
            /* title_font_px: */ 16,
            /* font_px: */ 14,
        )
    } else {
        0
    };

    // ----------------------------
    // 3) Split drawing areas
    // ----------------------------
    let (plot_area, legend_area_opt): (DrawingArea<DB, Shift>, Option<DrawingArea<DB, Shift>>) =
        match legend {
            LegendMode::Right => {
                let (plot, legend) = root.split_horizontally((85).percent_width());
                (plot, Some(legend))
            }
            LegendMode::Top => {
                let h = legend_needed_h.max(40);
                let (legend, plot) = root.split_vertically(h);
                (plot, Some(legend))
            }
            LegendMode::Bottom => {
                let h = legend_needed_h.max(40);
                // keep at least 40px for plot area
                let (plot, legend) = root.split_vertically((root_h - h).max(40));
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

    // ----------------------------
    // 4) Build chart (scaled Y range)
    // ----------------------------
    let mut chart = ChartBuilder::on(&plot_area)
        .margin(MARGIN as u32)
        .caption(
            {
                let t = title.trim();
                if t.is_empty() || t == "World Bank Indicator(s)" {
                    // derive from indicator names
                    let names: BTreeSet<&str> =
                        points.iter().map(|p| p.indicator_name.as_str()).collect();
                    if names.is_empty() {
                        "World Bank Series".to_string()
                    } else if names.len() == 1 {
                        names.iter().next().unwrap().to_string()
                    } else if names.len() <= 3 {
                        names.into_iter().collect::<Vec<_>>().join(", ")
                    } else {
                        let first = names.iter().next().unwrap();
                        let more = names.len() - 1;
                        format!("{first} + {more} more")
                    }
                } else {
                    t.to_string()
                }
            },
            (FontFamily::SansSerif, 24),
        )
        .set_label_area_size(LabelAreaPosition::Left, left_label_width_px)
        .set_label_area_size(LabelAreaPosition::Bottom, 56)
        .build_cartesian_2d(x_min..x_max, (min_val / yscale)..(max_val / yscale))
        .map_err(|e| anyhow::anyhow!("{:?}", e))?;

    chart
        .configure_mesh()
        .x_desc("Year")
        .y_desc(y_axis_title)
        .x_labels(x_label_count)
        .y_labels(y_label_count)
        .x_label_formatter(&x_label_fmt)
        .y_label_formatter(&y_label_fmt_scaled)
        .label_style((FontFamily::SansSerif, 12))
        .axis_desc_style((FontFamily::SansSerif, 16))
        .draw()
        .map_err(|e| anyhow::anyhow!("{:?}", e))?;

    // ----------------------------
    // 5) Draw series & collect legend items
    // ----------------------------
    let mut legend_items: Vec<(String, RGBAColor)> = Vec::new();
    let inside_mode = matches!(legend, LegendMode::Inside);

    // Create a flag for easier handling
    let use_country_styles = country_styles.unwrap_or(false);

    // Pre-compute unique countries for consistent ordering (if using country styles)
    let country_list: Vec<String> = if use_country_styles {
        let unique_countries: std::collections::BTreeSet<String> = series_list
            .iter()
            .map(|(iso3, _, _, _, _)| iso3.clone())
            .collect();
        unique_countries.into_iter().collect()
    } else {
        Vec::new()
    };

    // Helper function to get the appropriate color for a series
    let get_series_color = |idx: usize, iso3: &str, indicator_id: &str| -> RGBAColor {
        // Use country-consistent styling if enabled
        if use_country_styles {
            if let Some(country_index) = country_list.iter().position(|c| c == iso3) {
                // Use the MS Office palette for base colors
                let base_colors = [
                    (68, 114, 196),   // blue
                    (237, 125, 49),   // orange
                    (165, 165, 165),  // gray
                    (255, 192, 0),    // gold
                    (91, 155, 213),   // light blue
                    (112, 173, 71),   // green
                    (38, 68, 120),    // dark blue
                    (158, 72, 14),    // dark orange
                    (99, 99, 99),     // dark gray
                    (153, 115, 0),    // brownish
                ];
                
                let base_color = base_colors[country_index % base_colors.len()];
                
                // Create brightness variation based on indicator
                use std::collections::hash_map::DefaultHasher;
                use std::hash::{Hash, Hasher};
                let mut hasher = DefaultHasher::new();
                indicator_id.hash(&mut hasher);
                let indicator_hash = hasher.finish();
                
                let brightness_factor = 0.7 + 0.6 * ((indicator_hash % 100) as f64 / 100.0);
                let adjusted_r = ((base_color.0 as f64 * brightness_factor).min(255.0).max(0.0)) as u8;
                let adjusted_g = ((base_color.1 as f64 * brightness_factor).min(255.0).max(0.0)) as u8;
                let adjusted_b = ((base_color.2 as f64 * brightness_factor).min(255.0).max(0.0)) as u8;
                
                return RGBAColor(adjusted_r, adjusted_g, adjusted_b, 1.0);
            }
        }
        
        // Default fallback: use index-based coloring
        office_color(idx)
    };

    match kind {
        PlotKind::Line
        | PlotKind::Scatter
        | PlotKind::LinePoints
        | PlotKind::Area
        | PlotKind::Loess => {
            for (idx, (iso3, indicator_id, country_label, indicator_label, series)) in
                series_list.iter().enumerate()
            {
                let color = get_series_color(idx, iso3, indicator_id);
                let base_label = make_label(country_label, indicator_label);
                let legend_label = if matches!(kind, PlotKind::Loess) {
                    format!("{base_label} (LOESS)")
                } else {
                    base_label
                };

                // Convert to f64 X and **scale Y**
                let series_f: Vec<(f64, f64)> = series
                    .iter()
                    .map(|(x, y)| (*x as f64, *y / yscale))
                    .collect();

                match kind {
                    PlotKind::Line => {
                        let style = ShapeStyle {
                            color,
                            filled: false,
                            stroke_width: 2,
                        };
                        let elem = chart
                            .draw_series(LineSeries::new(series_f.clone(), style))
                            .map_err(|e| anyhow::anyhow!("{:?}", e))?;
                        if inside_mode {
                            let legend_color = color;
                            let legend_text = legend_label.clone();
                            elem.label(legend_text.clone()).legend(move |(x, y)| {
                                EmptyElement::at((x, y))
                                    + Circle::new((x + 8, y), 4, legend_color.clone().filled())
                                    + Text::new(
                                        legend_text.clone(),
                                        (x + 20, y),
                                        (FontFamily::SansSerif, 14),
                                    )
                            });
                        } else {
                            legend_items.push((legend_label, color));
                        }
                    }
                    PlotKind::Scatter => {
                        let elem = chart
                            .draw_series(
                                series_f
                                    .iter()
                                    .map(|(x, y)| Circle::new((*x, *y), 3, color.clone().filled())),
                            )
                            .map_err(|e| anyhow::anyhow!("{:?}", e))?;
                        if inside_mode {
                            let legend_color = color;
                            let legend_text = legend_label.clone();
                            elem.label(legend_text.clone()).legend(move |(x, y)| {
                                EmptyElement::at((x, y))
                                    + Circle::new((x + 8, y), 4, legend_color.clone().filled())
                                    + Text::new(
                                        legend_text.clone(),
                                        (x + 20, y),
                                        (FontFamily::SansSerif, 14),
                                    )
                            });
                        } else {
                            legend_items.push((legend_label, color));
                        }
                    }
                    PlotKind::LinePoints => {
                        let style = ShapeStyle {
                            color,
                            filled: false,
                            stroke_width: 2,
                        };
                        chart
                            .draw_series(LineSeries::new(series_f.clone(), style))
                            .map_err(|e| anyhow::anyhow!("{:?}", e))?;
                        let elem = chart
                            .draw_series(
                                series_f
                                    .iter()
                                    .map(|(x, y)| Circle::new((*x, *y), 3, color.clone().filled())),
                            )
                            .map_err(|e| anyhow::anyhow!("{:?}", e))?;
                        if inside_mode {
                            let legend_color = color;
                            let legend_text = legend_label.clone();
                            elem.label(legend_text.clone()).legend(move |(x, y)| {
                                EmptyElement::at((x, y))
                                    + Circle::new((x + 8, y), 4, legend_color.clone().filled())
                                    + Text::new(
                                        legend_text.clone(),
                                        (x + 20, y),
                                        (FontFamily::SansSerif, 14),
                                    )
                            });
                        } else {
                            legend_items.push((legend_label, color));
                        }
                    }
                    PlotKind::Area => {
                        let baseline_scaled = 0.0f64.min(min_val) / yscale;
                        let fill = color.clone().mix(0.20).filled();
                        let border = color.clone().stroke_width(1);
                        let elem = chart
                            .draw_series(
                                AreaSeries::new(series_f.clone(), baseline_scaled, fill)
                                    .border_style(border),
                            )
                            .map_err(|e| anyhow::anyhow!("{:?}", e))?;
                        if inside_mode {
                            let legend_color = color;
                            let legend_text = legend_label.clone();
                            elem.label(legend_text.clone()).legend(move |(x, y)| {
                                EmptyElement::at((x, y))
                                    + Circle::new((x + 8, y), 4, legend_color.clone().filled())
                                    + Text::new(
                                        legend_text.clone(),
                                        (x + 20, y),
                                        (FontFamily::SansSerif, 14),
                                    )
                            });
                        } else {
                            legend_items.push((legend_label, color));
                        }
                    }
                    PlotKind::Loess => {
                        // Smooth on original values, then **scale** the result for plotting
                        let xs: Vec<f64> = series.iter().map(|(x, _)| *x as f64).collect();
                        let ys: Vec<f64> = series.iter().map(|(_, y)| *y).collect();
                        let yhat = loess_series(&xs, &ys, loess_span);
                        let smoothed: Vec<(f64, f64)> = xs
                            .into_iter()
                            .zip(yhat.into_iter().map(|v| v / yscale))
                            .collect();
                        let style = ShapeStyle {
                            color,
                            filled: false,
                            stroke_width: 3,
                        };
                        let elem = chart
                            .draw_series(LineSeries::new(smoothed, style))
                            .map_err(|e| anyhow::anyhow!("{:?}", e))?;
                        if inside_mode {
                            let legend_color = color;
                            let legend_text = legend_label.clone();
                            elem.label(legend_text.clone()).legend(move |(x, y)| {
                                EmptyElement::at((x, y))
                                    + Circle::new((x + 8, y), 4, legend_color.clone().filled())
                                    + Text::new(
                                        legend_text.clone(),
                                        (x + 20, y),
                                        (FontFamily::SansSerif, 14),
                                    )
                            });
                        } else {
                            legend_items.push((legend_label, color));
                        }
                    }
                    _ => {}
                }
            }
        }
        PlotKind::StackedArea => {
            let years_all: Vec<i32> = (min_year..=max_year).collect();
            let mut cum: Vec<f64> = vec![0.0; years_all.len()];

            for (idx, (iso3, indicator_id, country_label, indicator_label, series)) in
                series_list.iter().enumerate()
            {
                let color = get_series_color(idx, iso3, indicator_id);
                let legend_label = make_label(country_label, indicator_label);

                // Map series to full year grid, missing -> 0.0
                let mut vals: Vec<f64> = vec![0.0; years_all.len()];
                for (y, v) in series.iter() {
                    if *y >= min_year && *y <= max_year {
                        vals[(*y - min_year) as usize] = (*v).max(0.0);
                    }
                }
                // Build upper curve by adding to cumulative
                let mut upper: Vec<(f64, f64)> = Vec::with_capacity(vals.len());
                let mut lower: Vec<(f64, f64)> = Vec::with_capacity(vals.len());
                for (i, v) in vals.iter().enumerate() {
                    let x = (min_year + i as i32) as f64;
                    lower.push((x, cum[i]));
                    cum[i] += *v;
                    upper.push((x, cum[i]));
                }
                // polygon: lower (forward) + upper (reverse), scaled
                let mut poly: Vec<(f64, f64)> = Vec::with_capacity(upper.len() * 2);
                poly.extend(lower.iter().map(|(x, y)| (*x, *y / yscale)));
                poly.extend(upper.iter().rev().map(|(x, y)| (*x, *y / yscale)));

                let fill = color.clone().mix(0.30).filled();
                let border = color.clone().stroke_width(1);
                chart
                    .draw_series(std::iter::once(Polygon::new(poly, fill)))
                    .map_err(|e| anyhow::anyhow!("{:?}", e))?;
                chart
                    .draw_series(std::iter::once(PathElement::new(
                        upper
                            .iter()
                            .map(|(x, y)| (*x, *y / yscale))
                            .collect::<Vec<_>>(),
                        border,
                    )))
                    .map_err(|e| anyhow::anyhow!("{:?}", e))?;

                legend_items.push((legend_label, color));
            }
        }
        PlotKind::GroupedBar => {
            let n_series = series_list.len().max(1);
            let group_width = 0.8f64;
            let bar_w = group_width / n_series as f64;

            for (idx, (iso3, indicator_id, country_label, indicator_label, series)) in
                series_list.iter().enumerate()
            {
                let color = get_series_color(idx, iso3, indicator_id);
                let legend_label = make_label(country_label, indicator_label);

                for (y, v) in series.iter() {
                    let x_center = *y as f64;
                    let x0 = x_center - group_width / 2.0 + idx as f64 * bar_w;
                    let x1 = x0 + bar_w;
                    let y0 = 0.0f64.min(*v) / yscale;
                    let y1 = 0.0f64.max(*v) / yscale;
                    let rect = Rectangle::new([(x0, y0), (x1, y1)], color.clone().filled());
                    chart
                        .draw_series(std::iter::once(rect))
                        .map_err(|e| anyhow::anyhow!("{:?}", e))?;
                }

                legend_items.push((legend_label, color));
            }
        }
    }

    // ----------------------------
    // 6) Legend rendering
    // ----------------------------
    if inside_mode {
        chart
            .configure_series_labels()
            .border_style(BLACK)
            .position(SeriesLabelPosition::UpperLeft)
            .background_style(WHITE.mix(0.85))
            .label_font((FontFamily::SansSerif, 14))
            .draw()
            .map_err(|e| anyhow::anyhow!("{:?}", e))?;
    } else if let Some(ref legend_area) = legend_area_opt {
        // Best practice: no explicit "Legend" title
        draw_legend_panel(legend_area, &legend_items, "", legend, axis_x_start_px)?;
    }

    // ----------------------------
    // 7) Present
    // ----------------------------
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
