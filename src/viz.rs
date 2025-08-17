//! Visualization utilities: render multi-series charts to **SVG** or **PNG**.
//!
//! - Distinct series colors (Microsoft Office palette)
//! - Locale-aware tick labels (`30,000` vs `30.000`), whole numbers
//! - Legend placement: `Inside`, `Right`, `Top`, `Bottom` (non-overlapping for external legends)
//! - Plot kinds: `Line`, `Scatter`, `LinePoints`, `Area`, `StackedArea`, `GroupedBar`, `Loess`
//! - Custom chart title and legend handling for long labels
///
/// Where to place the legend.
/// - `Inside`: overlay inside the plot (may overlap data)
/// - `Right`: separate right-side panel (no overlap)
/// - `Top`/`Bottom`: separate bands that wrap long labels
/// #[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// pub enum LegendMode {/* … */}
///
/// Which chart to render.
/// - `Line`: polyline per series
/// - `Scatter`: markers only
/// - `LinePoints`: line + markers
/// - `Area`: filled area to baseline
/// - `StackedArea`: positive stacking by year across series
/// - `GroupedBar`: per-year grouped bars (one per series)
/// - `Loess`: locally weighted regression (span controls smoothness)
/// #[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// pub enum PlotKind {/* … */}
///
/// Fully-configurable chart renderer.
///
/// ### Arguments
/// - `points`: tidy rows (`DataPoint`). Rows with `year == 0` or `value == None` are ignored for plotting.
/// - `out_path`: file path ending in `.svg` or `.png`
/// - `width`, `height`: pixel size
/// - `locale_tag`: e.g. `"en"`, `"de"`, `"fr"`
/// - `legend`: legend placement strategy (`LegendMode`)
/// - `title`: chart title
/// - `kind`: chart type (`PlotKind`)
/// - `loess_span`: fraction `(0, 1]` used **only** for `PlotKind::Loess`
///
/// ### Behavior
/// - The X axis is the **year**; the Y axis is the numeric `value`.
/// - Axis labels use thousands-grouping from `num-format`.
/// - When min/max on an axis are equal, the range is widened slightly to keep Plotters happy.
///
/// ### Example
/// ```no_run
/// # use world_bank_data_rust::viz::{self, LegendMode, PlotKind};
/// # use world_bank_data_rust::models::DataPoint;
/// let data: Vec<DataPoint> = vec![]; // fill with observations
/// viz::plot_chart(
///     &data, "chart.svg", 1000, 600, "en",
///     LegendMode::Right, "My Chart", PlotKind::LinePoints, 0.3
/// )?;
/// # Ok::<(), anyhow::Error>(())
/// ```
/// pub fn plot_chart<P: AsRef<std::path::Path>>(/* … */) -> anyhow::Result<()> { /* … */
/// }
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

/// Plot types supported by this module.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlotKind {
    /// Multi-series line chart (default).
    Line,
    /// Scatter (markers only).
    Scatter,
    /// Line + markers overlay.
    LinePoints,
    /// Area chart (filled area from baseline to values).
    Area,
    /// Stacked area chart (positive values stacked upward).
    StackedArea,
    /// Grouped bar chart (per year, bars per series).
    GroupedBar,
    /// LOESS smoothed line (span parameter controls smoothness).
    Loess,
}

/// Microsoft Office (2013+) chart series palette.
/// Order: Blue, Orange, Gray, Gold, Light Blue, Green, Dark Blue, Dark Orange, Dark Gray, Brownish Gold.
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

/// Convenience: plot with default locale (`"en"`) and default legend (`Right`) as a line chart.
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
        LegendMode::Right,
        "World Bank Indicator(s)",
        PlotKind::Line,
        0.3, // default LOESS span
    )
}

/// Convenience: plot with chosen locale and default legend (`Right`) as a line chart.
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
        LegendMode::Right,
        "World Bank Indicator(s)",
        PlotKind::Line,
        0.3,
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
    )
}

/// Fully-configurable entry point: choose locale, legend placement, custom title, plot kind, and LOESS span.
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
            root, points, min_year, max_year, min_val, max_val, num_locale, legend, title, kind,
            loess_span,
        )?;
    } else {
        let root = BitMapBackend::new(path_string.as_str(), (width, height)).into_drawing_area();
        draw_chart(
            root, points, min_year, max_year, min_val, max_val, num_locale, legend, title, kind,
            loess_span,
        )?;
    }
    Ok(())
}

/// Heuristic: estimate pixel width of text (Plotters has no built-in text measuring).
fn estimate_text_width_px(text: &str, font_px: u32) -> u32 {
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
            let mut y = start_y;
            for (label, color) in items {
                let max_text_w = w.saturating_sub(48);
                let text = truncate_to_width(label, font_px, max_text_w);
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
            let mut x = pad_x;
            let mut y = start_y;
            for (label, color) in items {
                let max_item_w = (w as i32 - x - pad_x).max(40) as u32;
                let text_max = max_item_w.saturating_sub(28);
                let text = truncate_to_width(label, font_px, text_max);
                let text_w = estimate_text_width_px(&text, font_px) as i32;
                let item_w = (28 + text_w + 12).min(w as i32 - 2 * pad_x);
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

/// Simple LOESS (locally weighted linear regression) smoother for yearly series.
/// `span` is the fraction of neighbors used (0 < span <= 1).
fn loess_series(xs: &[f64], ys: &[f64], span: f64) -> Vec<f64> {
    let n = xs.len();
    if n == 0 {
        return vec![];
    }
    let span = span.clamp(1.0 / n as f64, 1.0);
    let window = ((n as f64 * span).ceil() as usize).max(2);
    let mut yhat = vec![0.0; n];
    for i in 0..n {
        // Find window of nearest neighbors around i
        let mut idx: Vec<usize> = (0..n).collect();
        idx.sort_by(|&a, &b| {
            (xs[a] - xs[i])
                .abs()
                .partial_cmp(&(xs[b] - xs[i]).abs())
                .unwrap()
        });
        let idxw = &idx[..window];
        let max_d = (xs[*idxw.last().unwrap()] - xs[i]).abs();
        // Weights: tricube kernel
        let mut sw = 0.0;
        let mut swx = 0.0;
        let mut swy = 0.0;
        let mut swxx = 0.0;
        let mut swxy = 0.0;
        for &j in idxw {
            let d = (xs[j] - xs[i]).abs();
            let u = if max_d == 0.0 {
                0.0
            } else {
                (d / max_d).min(1.0)
            };
            let w = (1.0 - u * u * u).powi(3);
            sw += w;
            swx += w * xs[j];
            swy += w * ys[j];
            swxx += w * xs[j] * xs[j];
            swxy += w * xs[j] * ys[j];
        }
        // Weighted linear regression y = a + b x
        let denom = sw * swxx - swx * swx;
        if denom.abs() < 1e-12 {
            yhat[i] = swy / sw.max(1e-12);
        } else {
            let b = (sw * swxy - swx * swy) / denom;
            let a = (swy - b * swx) / sw;
            yhat[i] = a + b * xs[i];
        }
    }
    yhat
}

/// Draw chart with chosen legend placement, custom title, plot kind, and LOESS span.
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
    kind: PlotKind,
    loess_span: f64,
) -> anyhow::Result<()>
where
    DB: DrawingBackend,
{
    // External legends split the drawing area so the legend never overlaps data.
    let (plot_area, legend_area_opt): (DrawingArea<DB, Shift>, Option<DrawingArea<DB, Shift>>) =
        match legend {
            LegendMode::Right => {
                let (plot, legend) = root.split_horizontally((85).percent_width());
                (plot, Some(legend))
            }
            LegendMode::Top => {
                let (legend, plot) = root.split_vertically(64);
                (plot, Some(legend))
            }
            LegendMode::Bottom => {
                let (plot, legend) = root.split_vertically((85).percent_height());
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

    // NOTE: we use f64 on X to allow grouped bar fractional offsets
    let x_min = min_year as f64;
    let x_max = max_year as f64;

    let mut chart = ChartBuilder::on(&plot_area)
        .margin(16)
        .caption(title, ("sans-serif", 24))
        .set_label_area_size(LabelAreaPosition::Left, 100)
        .set_label_area_size(LabelAreaPosition::Bottom, 56)
        .build_cartesian_2d(x_min..x_max, min_val..max_val)
        .map_err(|e| anyhow::anyhow!("{:?}", e))?;

    // Axis label formatters: integers with locale thousands separators
    let y_label_fmt = |v: &f64| {
        let n = (*v).round() as i64;
        n.to_formatted_string(num_locale)
    };
    let x_label_fmt = |x: &f64| (x.round() as i32).to_string();

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
        .label_style(("sans-serif", 12))
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

    // Prepare legend items
    let mut legend_items: Vec<(String, RGBAColor)> = Vec::new();
    let inside_mode = matches!(legend, LegendMode::Inside);

    // Distinct behaviors by plot kind
    match kind {
        PlotKind::Line
        | PlotKind::Scatter
        | PlotKind::LinePoints
        | PlotKind::Area
        | PlotKind::Loess => {
            for (idx, ((country_iso3, indicator_id), series)) in groups.iter().enumerate() {
                let color = office_color(idx);
                let country_label = country_name_by_iso3
                    .get(country_iso3)
                    .cloned()
                    .unwrap_or_else(|| country_iso3.clone());
                let indicator_label = indicator_name_by_id
                    .get(indicator_id)
                    .cloned()
                    .unwrap_or_else(|| indicator_id.clone());
                let legend_label = format!("{} — {}", country_label, indicator_label);

                // Convert to f64 X to draw on float axis
                let series_f: Vec<(f64, f64)> =
                    series.iter().map(|(x, y)| (*x as f64, *y)).collect();

                match kind {
                    PlotKind::Line => {
                        let style = ShapeStyle {
                            color: color,
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
                                        ("sans-serif", 14),
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
                                        ("sans-serif", 14),
                                    )
                            });
                        } else {
                            legend_items.push((legend_label, color));
                        }
                    }
                    PlotKind::LinePoints => {
                        let style = ShapeStyle {
                            color: color,
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
                                        ("sans-serif", 14),
                                    )
                            });
                        } else {
                            legend_items.push((legend_label, color));
                        }
                    }
                    PlotKind::Area => {
                        let baseline = 0.0f64.min(min_val);
                        let fill = color.clone().mix(0.20).filled();
                        let border = color.clone().stroke_width(1);
                        let elem = chart
                            .draw_series(
                                AreaSeries::new(series_f.clone(), baseline, fill)
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
                                        ("sans-serif", 14),
                                    )
                            });
                        } else {
                            legend_items.push((legend_label, color));
                        }
                    }
                    PlotKind::Loess => {
                        let xs: Vec<f64> = series_f.iter().map(|(x, _)| *x).collect();
                        let ys: Vec<f64> = series_f.iter().map(|(_, y)| *y).collect();
                        let yhat = loess_series(&xs, &ys, loess_span);
                        let smoothed: Vec<(f64, f64)> =
                            xs.into_iter().zip(yhat.into_iter()).collect();
                        let style = ShapeStyle {
                            color: color,
                            filled: false,
                            stroke_width: 3,
                        };
                        let elem = chart
                            .draw_series(LineSeries::new(smoothed, style))
                            .map_err(|e| anyhow::anyhow!("{:?}", e))?;
                        if inside_mode {
                            let legend_color = color;
                            let legend_text =
                                format!("{} — {} (LOESS)", country_label, indicator_label);
                            elem.label(legend_text.clone()).legend(move |(x, y)| {
                                EmptyElement::at((x, y))
                                    + Circle::new((x + 8, y), 4, legend_color.clone().filled())
                                    + Text::new(
                                        legend_text.clone(),
                                        (x + 20, y),
                                        ("sans-serif", 14),
                                    )
                            });
                        } else {
                            legend_items.push((
                                format!("{} — {} (LOESS)", country_label, indicator_label),
                                color,
                            ));
                        }
                    }
                    _ => {}
                }
            }
        }
        PlotKind::StackedArea => {
            // Build a union of years
            let years_all: Vec<i32> = (min_year..=max_year).collect();
            // Prepare cumulative stacks
            let mut cum: Vec<f64> = vec![0.0; years_all.len()];
            for (idx, ((_country_iso3, _indicator_id), _)) in groups.iter().enumerate() {
                let _ = idx; // to compute colors in second loop
            }
            // To keep legend order consistent, iterate groups in order
            for (idx, ((country_iso3, indicator_id), series)) in groups.iter().enumerate() {
                let color = office_color(idx);
                let country_label = country_name_by_iso3
                    .get(country_iso3)
                    .cloned()
                    .unwrap_or_else(|| country_iso3.clone());
                let indicator_label = indicator_name_by_id
                    .get(indicator_id)
                    .cloned()
                    .unwrap_or_else(|| indicator_id.clone());
                let legend_label = format!("{} — {}", country_label, indicator_label);

                // Map series to full year grid, missing -> 0.0
                let mut vals: Vec<f64> = vec![0.0; years_all.len()];
                for (y, v) in series.iter() {
                    if *y >= min_year && *y <= max_year {
                        vals[(*y - min_year) as usize] = (*v).max(0.0); // stack positive part
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
                // Create polygon points: lower (forward) + upper (reverse)
                let mut poly: Vec<(f64, f64)> = Vec::with_capacity(upper.len() * 2);
                poly.extend(lower.iter().cloned());
                poly.extend(upper.iter().rev().cloned());

                let fill = color.clone().mix(0.30).filled();
                let border = color.clone().stroke_width(1);
                // Use an AreaSeries from lower baseline; emulate by drawing polygon
                chart
                    .draw_series(std::iter::once(Polygon::new(poly, fill)))
                    .map_err(|e| anyhow::anyhow!("{:?}", e))?;
                chart
                    .draw_series(std::iter::once(PathElement::new(upper.clone(), border)))
                    .map_err(|e| anyhow::anyhow!("{:?}", e))?;

                if inside_mode {
                    // Collect, then render with built-in
                    legend_items.push((legend_label, color));
                } else {
                    legend_items.push((legend_label, color));
                }
            }
        }
        PlotKind::GroupedBar => {
            let n_series = groups.len().max(1);
            let group_width = 0.8f64; // width of a full year band
            let bar_w = group_width / n_series as f64;

            for (idx, ((country_iso3, indicator_id), series)) in groups.iter().enumerate() {
                let color = office_color(idx);
                let country_label = country_name_by_iso3
                    .get(country_iso3)
                    .cloned()
                    .unwrap_or_else(|| country_iso3.clone());
                let indicator_label = indicator_name_by_id
                    .get(indicator_id)
                    .cloned()
                    .unwrap_or_else(|| indicator_id.clone());
                let legend_label = format!("{} — {}", country_label, indicator_label);

                for (y, v) in series.iter() {
                    let x_center = *y as f64;
                    let x0 = x_center - group_width / 2.0 + idx as f64 * bar_w;
                    let x1 = x0 + bar_w;
                    let y0 = 0.0f64.min(*v);
                    let y1 = 0.0f64.max(*v);
                    let rect = Rectangle::new([(x0, y0), (x1, y1)], color.clone().filled());
                    chart
                        .draw_series(std::iter::once(rect))
                        .map_err(|e| anyhow::anyhow!("{:?}", e))?;
                }

                if inside_mode {
                    legend_items.push((legend_label, color));
                } else {
                    legend_items.push((legend_label, color));
                }
            }
        }
    }

    if inside_mode {
        chart
            .configure_series_labels()
            .border_style(BLACK)
            .position(SeriesLabelPosition::UpperLeft)
            .background_style(WHITE.mix(0.85))
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
