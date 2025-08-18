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
///
/// #[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// pub enum LegendMode {/* … */}
///
/// Which chart to render.
///
/// - `Line`: polyline per series
/// - `Scatter`: markers only
/// - `LinePoints`: line + markers
/// - `Area`: filled area to baseline
/// - `StackedArea`: positive stacking by year across series
/// - `GroupedBar`: per-year grouped bars (one per series)
/// - `Loess`: locally weighted regression (span controls smoothness)
///
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
use plotters::series::{AreaSeries, LineSeries}; // <-- series types (features enabled above)
use plotters::style::text_anchor::{HPos, Pos, VPos};
use plotters::style::{FontFamily, FontStyle};

use plotters_bitmap::BitMapBackend;
use plotters_svg::SVGBackend; // <-- SVG backend // <-- Bitmap backend with `image` feature

use std::collections::BTreeSet;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use std::sync::Once;

/// One-time registration for a fallback "sans-serif" font when using the `ab_glyph` text path.
/// Required because `ab_glyph` doesn't discover OS fonts.
static INIT_FONTS: Once = Once::new();

fn ensure_fonts_registered() {
    // Safe to call many times; only runs once.
    INIT_FONTS.call_once(|| {
        // If you move/rename the file, adjust the relative path.
        // From `src/viz.rs` → project root → `assets/DejaVuSans.ttf`
        let _ = plotters::style::register_font(
            "sans-serif",
            plotters::style::FontStyle::Normal,
            include_bytes!("../assets/DejaVuSans.ttf"),
        );
        // If you also add bold/italic files, you can register them too:
        // let _ = plotters::style::register_font("sans-serif", plotters::style::FontStyle::Bold, include_bytes!("../assets/DejaVuSans-Bold.ttf"));
        // let _ = plotters::style::register_font("sans-serif", plotters::style::FontStyle::Italic, include_bytes!("../assets/DejaVuSans-Oblique.ttf"));
    });
}

/// Default legend placement following mainstream design guidance:
/// - Horizontal legend **below** the chart works well for dashboards and keeps labels close
///   to the x-axis start.
/// References: IBM Carbon (bottom/top as default), U.S. Gov Data Viz Standards.
/// (You can still override per call.)
pub const DEFAULT_LEGEND_MODE: LegendMode = LegendMode::Bottom;

/// Compute a tight left label area width for the Y axis (in pixels),
/// based on the formatted tick labels that will appear.
/// - `ymin_scaled..ymax_scaled`: the **scaled** Y range you pass to Plotters
/// - `ticks`: how many Y labels you plan to show (e.g., 10)
/// - `font_px`: font size used for axis labels (e.g., 12)
///
/// Returns a width clamped to a sensible range to avoid extremes.
fn compute_left_label_area_px(
    ymin_scaled: f64,
    ymax_scaled: f64,
    ticks: usize,
    font_px: u32,
) -> u32 {
    // This must match the formatter you use in .configure_mesh().y_label_formatter(...)
    let y_label_fmt = |v: f64| {
        let a = v.abs();
        let prec = if a >= 100.0 {
            0
        } else if a >= 10.0 {
            1
        } else {
            2
        };
        format!("{:.*}", prec, v)
    };

    let mut max_px = 0u32;
    // Sample the same number of tick positions as you request from Plotters.
    for i in 0..=ticks {
        let t = if ticks == 0 {
            0.0
        } else {
            i as f64 / ticks as f64
        };
        let v = ymin_scaled + (ymax_scaled - ymin_scaled) * t;
        let s = y_label_fmt(v);
        max_px = max_px.max(estimate_text_width_px(&s, font_px));
    }

    // Add padding for tick marks & a little breathing room.
    // Clamp to avoid silly extremes; tune these if you like.
    let with_padding = max_px.saturating_add(18);
    with_padding.clamp(48, 140)
}

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

/// Pick a single Y-axis scale and its human label based on the overall magnitude.
/// Returns (scale, label), e.g. (1e6, "millions").
fn choose_axis_scale(max_abs: f64) -> (f64, &'static str) {
    if max_abs >= 1.0e12 {
        (1.0e12, "trillions")
    } else if max_abs >= 1.0e9 {
        (1.0e9, "billions")
    } else if max_abs >= 1.0e6 {
        (1.0e6, "millions")
    } else if max_abs >= 1.0e3 {
        (1.0e3, "thousands")
    } else {
        (1.0, "")
    }
}

/// Try to extract a unit from the indicator name, e.g. "GDP (current US$)" -> "current US$".
fn extract_unit_from_indicator_name(name: &str) -> Option<String> {
    let open = name.rfind('(')?;
    let close = name.rfind(')')?;
    if close > open {
        let inner = name[open + 1..close].trim();
        if !inner.is_empty() {
            Some(inner.to_string())
        } else {
            None
        }
    } else {
        None
    }
}

/// If (and only if) a single indicator is shown, derive a common unit string from its name.
fn derive_axis_unit(points: &[DataPoint]) -> Option<String> {
    use std::collections::BTreeSet;
    let mut names: BTreeSet<&str> = points.iter().map(|p| p.indicator_name.as_str()).collect();
    if names.len() == 1 {
        extract_unit_from_indicator_name(names.iter().next().unwrap())
    } else {
        None
    }
}

/// Heuristic: treat percent-like units as non-scalable (no thousands/millions/billions).
fn is_percentage_like(unit: &str) -> bool {
    let u = unit.to_ascii_lowercase();
    u.contains('%') || u.contains("percent") || u.contains("percentage") || u.contains("per cent")
}

/// Greedy word-wrap using the same width heuristic as the chart.
/// Falls back to character breaks for very long unbroken words.
fn wrap_text_to_width(text: &str, font_px: u32, max_px: u32) -> Vec<String> {
    if max_px <= 12 {
        return vec![truncate_to_width(text, font_px, max_px)];
    }
    let mut lines: Vec<String> = Vec::new();
    let mut cur = String::new();
    for word in text.split_whitespace() {
        let candidate = if cur.is_empty() {
            word.to_string()
        } else {
            format!("{cur} {word}")
        };
        if estimate_text_width_px(&candidate, font_px) <= max_px {
            cur = candidate;
        } else if cur.is_empty() {
            // Single long word: hard-break by characters
            let mut buf = String::new();
            for ch in word.chars() {
                let cand = format!("{buf}{ch}");
                if estimate_text_width_px(&cand, font_px) > max_px {
                    if buf.is_empty() {
                        lines.push(truncate_to_width(word, font_px, max_px));
                        buf.clear();
                        break;
                    } else {
                        lines.push(buf);
                        buf = ch.to_string();
                    }
                } else {
                    buf = cand;
                }
            }
            if !buf.is_empty() {
                lines.push(buf);
            }
        } else {
            lines.push(cur);
            cur = word.to_string();
        }
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    lines
}

/// Estimate how tall the TOP/BOTTOM legend band must be to fit all items,
/// honoring wrapping and multi-row flow. Returns pixels.
fn estimate_top_bottom_legend_height_px(
    labels: &[String],
    start_x: i32, // where first text column should start (aligns to plot’s X-axis)
    total_w: i32, // full canvas width in pixels
    has_title: bool,
    title_font_px: u32,
    font_px: u32,
) -> i32 {
    // Must match draw_legend_panel’s constants/logic
    let line_h: i32 = font_px as i32 + 4;
    let row_gap: i32 = 6;
    let pad_small: i32 = 6;
    let pad_band: i32 = 8;
    let marker_radius: i32 = 4;
    let marker_to_text_gap: i32 = 12;
    let trailing_gap: i32 = 12;

    let mut height = if has_title {
        pad_band + title_font_px as i32 + 8 // title + gap
    } else {
        pad_band + 8
    };

    // Current row state
    let mut x = start_x;
    let mut row_h: i32 = line_h; // tallest block on current row

    // Maximum usable width for one row (right padding = pad_small)
    let usable_row_w = total_w - pad_small;
    // Maximum text width on a fresh row (reserve space for dot/gap/trailing)
    let text_max_fresh = (usable_row_w - start_x)
        .max(40)
        .saturating_sub((marker_to_text_gap + marker_radius + trailing_gap) as i32)
        as u32;

    for label in labels {
        let full_text_w = estimate_text_width_px(label, font_px) as i32;
        let mut block_w = marker_to_text_gap + marker_radius + full_text_w + trailing_gap;
        let mut block_h = line_h;

        // If the single item won't fit even on a fresh row, wrap to multiple lines.
        if block_w > (usable_row_w - start_x) {
            let lines = wrap_text_to_width(label, font_px, text_max_fresh);
            let max_line_w = lines
                .iter()
                .map(|s| estimate_text_width_px(s, font_px) as i32)
                .max()
                .unwrap_or(0);
            block_w = marker_to_text_gap + marker_radius + max_line_w + trailing_gap;
            block_h = (lines.len().max(1) as i32) * line_h;
        }

        // If it doesn't fit on this row, start a new row
        if x + block_w > usable_row_w {
            height += row_h + row_gap;
            x = start_x;
            row_h = block_h;
        } else {
            row_h = row_h.max(block_h);
        }
        // Advance x (flow)
        x += block_w;
    }

    // Add last row + bottom padding
    height += row_h + pad_band;
    height.clamp(40, total_w) // conservative clamp; you can lift the upper bound if needed
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
        DEFAULT_LEGEND_MODE,
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
/// Spacing rules:
/// - Top/Bottom: text starts aligned with the plot’s X-axis start (first item),
///   and subsequent items flow horizontally using the current `x`.
/// - Right: a small gutter keeps it visually close to the plot.
/// Truncation only happens if an item cannot fit even on a fresh line.
fn draw_legend_panel<DB: DrawingBackend>(
    legend_area: &DrawingArea<DB, Shift>,
    items: &[(String, RGBAColor)],
    title: &str, // pass "" to omit (recommended)
    placement: LegendMode,
    axis_x_start_px: i32, // plot’s X-axis start (from root’s left edge)
) -> anyhow::Result<()> {
    legend_area
        .fill(&WHITE)
        .map_err(|e| anyhow::anyhow!("{:?}", e))?;

    let (w_u32, _) = legend_area.dim_in_pixel();
    let w = w_u32 as i32;

    // Layout
    let font_px: u32 = 14; // row font size
    let line_h: i32 = font_px as i32 + 4; // per-line height for wrapped text
    let row_gap: i32 = 6; // gap between legend items
    let pad_small: i32 = 6; // small gutter/padding
    let pad_band: i32 = 8; // inner padding for top/bottom bands
    let marker_radius: i32 = 4;
    let marker_to_text_gap: i32 = 12;
    let trailing_gap: i32 = 12;

    // Styles
    let has_title = !title.trim().is_empty();
    let title_font_px: u32 = 16;
    let title_style: TextStyle = TextStyle::from((FontFamily::SansSerif, title_font_px))
        .pos(Pos::new(HPos::Left, VPos::Top));
    let label_style_center: TextStyle =
        TextStyle::from((FontFamily::SansSerif, font_px)).pos(Pos::new(HPos::Left, VPos::Center));
    let label_style_top: TextStyle =
        TextStyle::from((FontFamily::SansSerif, font_px)).pos(Pos::new(HPos::Left, VPos::Top));

    match placement {
        LegendMode::Right => {
            // Small gutter keeps it close to the plot
            let pad_x: i32 = 6;

            // Title (optional)
            let mut y = if has_title {
                let title_y_top = pad_small;
                legend_area
                    .draw(&Text::new(title, (pad_x, title_y_top), title_style.clone()))
                    .map_err(|e| anyhow::anyhow!("{:?}", e))?;
                title_y_top + title_font_px as i32 + 8
            } else {
                pad_small + 6
            };

            // Available width for text on the right panel
            let text_x = pad_x + 24; // 12 (dot center) + 12 (gap)
            let max_text_w = (w - text_x - pad_x).max(40) as u32;

            // Use CENTER vertical anchoring for each line to align with the dot
            let label_style_center: TextStyle = TextStyle::from((FontFamily::SansSerif, font_px))
                .pos(Pos::new(HPos::Left, VPos::Center));

            // Optional 1px vertical nudge if your font renders visually low/high.
            // Set to 0 if you don't want any tweak.
            let valign_fudge: i32 = 0; // try 1 or -1 if you still see a slight offset

            for (label, color) in items {
                // Wrap label to the available width (no truncation unless absolutely necessary)
                let lines = wrap_text_to_width(label, font_px, max_text_w);
                let block_h = (lines.len().max(1) as i32) * line_h;

                // Dot centered to the text block
                let marker_x = pad_x + 12;
                let block_center_y = y + block_h / 2 + valign_fudge;

                legend_area
                    .draw(&Circle::new(
                        (marker_x, block_center_y),
                        marker_radius,
                        color.clone().filled(),
                    ))
                    .map_err(|e| anyhow::anyhow!("{:?}", e))?;

                // Draw each wrapped line centered on its own line box
                for (i, line) in lines.iter().enumerate() {
                    let line_center_y = y + (i as i32) * line_h + line_h / 2 + valign_fudge;
                    legend_area
                        .draw(&Text::new(
                            line.as_str(),
                            (text_x, line_center_y),
                            label_style_center.clone(),
                        ))
                        .map_err(|e| anyhow::anyhow!("{:?}", e))?;
                }

                y += block_h + row_gap;
            }
        }

        LegendMode::Top | LegendMode::Bottom => {
            let start_x = axis_x_start_px;

            // Metrics (must match estimator)
            let line_h: i32 = font_px as i32 + 4;
            let row_gap: i32 = 6;
            let pad_small: i32 = 6;
            let pad_band: i32 = 8;
            let marker_radius: i32 = 4;
            let marker_to_text_gap: i32 = 12;
            let trailing_gap: i32 = 12;

            // First row center (below optional title)
            let mut x = start_x;
            let mut y_center = if !title.trim().is_empty() {
                let title_y_top = pad_band;
                legend_area
                    .draw(&Text::new(
                        title,
                        (start_x, title_y_top),
                        title_style.clone(),
                    ))
                    .map_err(|e| anyhow::anyhow!("{:?}", e))?;
                title_y_top + 16 + 8 + (line_h / 2) // title 16px + 8px gap + half line height
            } else {
                pad_band + 8 + (line_h / 2)
            };

            let (w_u32, _) = legend_area.dim_in_pixel();
            let w = w_u32 as i32;
            let usable_row_w = w - pad_small;
            let label_style_center: TextStyle = TextStyle::from((FontFamily::SansSerif, font_px))
                .pos(Pos::new(HPos::Left, VPos::Center));

            // Max text width if item occupies a fresh row
            let text_max_fresh = (usable_row_w - start_x)
                .max(40)
                .saturating_sub((marker_to_text_gap + marker_radius + trailing_gap) as i32)
                as u32;

            for (label, color) in items {
                let full_text_w = estimate_text_width_px(label, font_px) as i32;
                let mut block_w = marker_to_text_gap + marker_radius + full_text_w + trailing_gap;
                let mut block_h = line_h;
                let mut lines: Option<Vec<String>> = None;

                // If the single item won't fit on a fresh row, wrap within the item
                if block_w > (usable_row_w - start_x) {
                    let wrapped = wrap_text_to_width(label, font_px, text_max_fresh);
                    let max_line_w = wrapped
                        .iter()
                        .map(|s| estimate_text_width_px(s, font_px) as i32)
                        .max()
                        .unwrap_or(0);
                    block_w = marker_to_text_gap + marker_radius + max_line_w + trailing_gap;
                    block_h = (wrapped.len().max(1) as i32) * line_h;
                    lines = Some(wrapped);
                }

                // If it doesn't fit in the current row, move to next row center
                if x + block_w > usable_row_w {
                    x = start_x;
                    y_center += block_h + row_gap;
                }

                // Draw item at (x, y_center)
                let text_x = x;
                let dot_x = (text_x - marker_to_text_gap).max(0);

                legend_area
                    .draw(&Circle::new(
                        (dot_x, y_center),
                        marker_radius,
                        color.clone().filled(),
                    ))
                    .map_err(|e| anyhow::anyhow!("{:?}", e))?;

                if let Some(lines) = lines {
                    // Multi-line block: center the whole block on y_center
                    let top = y_center - block_h / 2;
                    for (i, ln) in lines.iter().enumerate() {
                        let line_center_y = top + (i as i32) * line_h + line_h / 2;
                        legend_area
                            .draw(&Text::new(
                                ln.as_str(),
                                (text_x, line_center_y),
                                label_style_center.clone(),
                            ))
                            .map_err(|e| anyhow::anyhow!("{:?}", e))?;
                    }
                } else {
                    // Single-line
                    legend_area
                        .draw(&Text::new(
                            label.as_str(),
                            (text_x, y_center),
                            label_style_center.clone(),
                        ))
                        .map_err(|e| anyhow::anyhow!("{:?}", e))?;
                }

                // Advance x for flow on this row
                x += block_w;
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

fn draw_chart<DB>(
    root: DrawingArea<DB, Shift>,
    points: &[DataPoint],
    min_year: i32,
    max_year: i32,
    min_val: f64,
    max_val: f64,
    num_locale: &num_format::Locale,
    legend: LegendMode,
    title: &str,
    kind: PlotKind,
    loess_span: f64,
) -> anyhow::Result<()>
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
    use std::collections::{BTreeMap, BTreeSet, HashMap};

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
    let has_title = false;
    let title_font_px: u32 = 16;
    let font_px: u32 = 14;

    // Inline estimator (closure) to avoid missing-symbol issues:
    let estimate_top_bottom_legend_height_px = |labels: &[String],
                                                start_x: i32,
                                                total_w: i32,
                                                has_title: bool,
                                                title_font_px: u32,
                                                font_px: u32|
     -> i32 {
        // Must mirror draw_legend_panel metrics
        let line_h: i32 = font_px as i32 + 4;
        let row_gap: i32 = 6;
        let pad_small: i32 = 6;
        let pad_band: i32 = 8;
        let marker_radius: i32 = 4;
        let marker_to_text_gap: i32 = 12;
        let trailing_gap: i32 = 12;

        let mut height = if has_title {
            pad_band + title_font_px as i32 + 8
        } else {
            pad_band + 8
        };

        let usable_row_w = total_w - pad_small;
        let mut x = start_x;
        let mut row_h: i32 = line_h;

        let text_max_fresh = (usable_row_w - start_x)
            .max(40)
            .saturating_sub((marker_to_text_gap + marker_radius + trailing_gap) as i32)
            as u32;

        for label in labels {
            let full_text_w = estimate_text_width_px(label, font_px) as i32;
            let mut block_w = marker_to_text_gap + marker_radius + full_text_w + trailing_gap;
            let mut block_h = line_h;

            if block_w > (usable_row_w - start_x) {
                let lines = wrap_text_to_width(label, font_px, text_max_fresh);
                let max_line_w = lines
                    .iter()
                    .map(|s| estimate_text_width_px(s, font_px) as i32)
                    .max()
                    .unwrap_or(0);
                block_w = marker_to_text_gap + marker_radius + max_line_w + trailing_gap;
                block_h = (lines.len().max(1) as i32) * line_h;
            }

            if x + block_w > usable_row_w {
                height += row_h + row_gap;
                x = start_x;
                row_h = block_h;
            } else {
                row_h = row_h.max(block_h);
            }
            x += block_w;
        }

        height += row_h + pad_band;
        height.clamp(40, (total_w * 2) / 3)
    };

    let legend_needed_h = if matches!(legend, LegendMode::Top | LegendMode::Bottom) {
        estimate_top_bottom_legend_height_px(
            &legend_texts,
            axis_x_start_px,
            root_w,
            has_title,
            title_font_px,
            font_px,
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
                    let mut names: BTreeSet<&str> =
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

    match kind {
        PlotKind::Line
        | PlotKind::Scatter
        | PlotKind::LinePoints
        | PlotKind::Area
        | PlotKind::Loess => {
            for (idx, (_iso3, _indicator_id, country_label, indicator_label, series)) in
                series_list.iter().enumerate()
            {
                let color = office_color(idx);
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

            for (idx, (_iso3, _indicator_id, country_label, indicator_label, series)) in
                series_list.iter().enumerate()
            {
                let color = office_color(idx);
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

            for (idx, (_iso3, _indicator_id, country_label, indicator_label, series)) in
                series_list.iter().enumerate()
            {
                let color = office_color(idx);
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
