//! Legend layout and drawing functions for external legend placement.

use anyhow::Result;
use plotters::backend::DrawingBackend;
use plotters::coord::Shift;
use plotters::prelude::*;
use plotters::style::FontFamily;
use plotters::style::text_anchor::{HPos, Pos, VPos};

use super::text::{estimate_text_width_px, wrap_text_to_width};
use super::types::LegendMode;
use crate::viz_style::{LineDash, MarkerShape};

/// Estimate how tall the TOP/BOTTOM legend band must be to fit all items,
/// honoring wrapping and multi-row flow. Returns pixels.
///
/// NOTE:
/// - This estimator mirrors the constants AND the table-layout flow logic used in draw_legend_panel()
///   for LegendMode::Top | LegendMode::Bottom to avoid clipping or excessive whitespace.
/// - Column widths are derived per column from the longest single-line label; if the total fits,
///   no wrapping is needed. Otherwise we fall back to uniform slots and wrap.
pub fn estimate_top_bottom_legend_height_px(
    labels: &[String],
    start_x: i32, // where first text column should start (aligns to plot's X-axis)
    total_w: i32, // full canvas width in pixels
    has_title: bool,
    title_font_px: u32,
    font_px: u32,
) -> i32 {
    // Must match draw_legend_panel()
    let line_h: i32 = font_px as i32 + 2; // tighter line height
    let row_gap: i32 = 4; // smaller vertical gap
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

    let usable_row_w = total_w - pad_small;

    // Pass 1: Greedy pack into rows to determine how many columns (K)
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut cur: Vec<String> = Vec::new();
    let mut x = start_x;

    let per_item_cap_px: i32 = ((usable_row_w - start_x) as f32 * 0.35).max(140.0) as i32;

    // Helper: compute a block width for a given label and text width cap (for packing phase)
    let block_width_for_cap = |label: &str, cap_px: i32| -> i32 {
        let cap = cap_px.max(40) as u32;
        let lines = wrap_text_to_width(label, font_px, cap);
        let max_line_w = lines
            .iter()
            .map(|s| estimate_text_width_px(s, font_px) as i32)
            .max()
            .unwrap_or(0);
        marker_to_text_gap + marker_radius + max_line_w + trailing_gap
    };

    for label in labels {
        let remaining_line_px = (usable_row_w - x).max(40);
        let text_cap_now = remaining_line_px - (marker_to_text_gap + marker_radius + trailing_gap);
        let text_cap_now = text_cap_now.min(per_item_cap_px);

        let mut block_w = block_width_for_cap(label, text_cap_now);

        if x + block_w > usable_row_w && !cur.is_empty() {
            rows.push(cur);
            cur = Vec::new();
            x = start_x;

            let fresh_text_cap = ((usable_row_w - start_x)
                - (marker_to_text_gap + marker_radius + trailing_gap))
                .min(per_item_cap_px);
            block_w = block_width_for_cap(label, fresh_text_cap);
        }

        x += block_w;
        cur.push(label.clone());
    }
    if !cur.is_empty() {
        rows.push(cur);
    }

    let k_cols = rows.iter().map(|r| r.len()).max().unwrap_or(1);

    // Compute per-column preferred widths from the longest single-line label in that column.
    // If the sum fits into the band, use these column widths; otherwise, fall back to uniform slots.
    let mut col_block_w: Vec<i32> = vec![60; k_cols]; // minimum sensible slot
    for row in rows.iter() {
        for (ci, label) in row.iter().enumerate() {
            // single-line text width (no wrapping)
            let text_w = estimate_text_width_px(label, font_px) as i32;
            let block_w = marker_to_text_gap + marker_radius + text_w + trailing_gap;
            if block_w > col_block_w[ci] {
                col_block_w[ci] = block_w;
            }
        }
    }
    let total_needed = start_x + col_block_w.iter().sum::<i32>();
    let available = usable_row_w;

    let slot_w_per_col: Vec<i32> = if total_needed <= available {
        // Use per-column widths; this allows long labels to stay on a single line when possible.
        col_block_w.clone()
    } else {
        // Fall back to uniform columns (previous behavior), which will trigger wrapping where needed.
        let uniform = ((usable_row_w - start_x) / (k_cols as i32)).max(60);
        vec![uniform; k_cols]
    };

    // Precompute per-column text caps (slot minus marker+gap+trailing)
    let text_cap_per_col: Vec<i32> = slot_w_per_col
        .iter()
        .map(|sw| (*sw - (marker_to_text_gap + marker_radius + trailing_gap)).max(40))
        .collect();

    // Pass 2: Estimate height row by row using the per-column caps (no drawing here)
    for (ri, row) in rows.iter().enumerate() {
        let mut row_max_h = line_h;
        for (ci, label) in row.iter().enumerate() {
            let cap = text_cap_per_col[ci] as u32;
            let lines = wrap_text_to_width(label, font_px, cap);
            let bh = (lines.len().max(1) as i32) * line_h;
            row_max_h = row_max_h.max(bh);
        }
        height += row_max_h;
        if ri + 1 < rows.len() {
            height += row_gap;
        }
    }

    height += pad_band;
    height
}

/// Draw the legend panel (Right: single column; Top/Bottom: table-like multi-row, column-aligned)
///
/// Table layout improvements:
/// - Compute per-column preferred widths from the longest single-line label in each column.
/// - If the sum fits the band width, use these widths so single-line labels do not wrap unnecessarily.
/// - Otherwise, fall back to uniform column widths and wrap as needed.
/// - Column x-positions are consistent across all rows, so entries align like a table.
pub fn draw_legend_panel<DB: DrawingBackend>(
    legend_area: &DrawingArea<DB, Shift>,
    items: &[(String, RGBAColor)],
    title: &str, // pass "" to omit (recommended)
    placement: LegendMode,
    axis_x_start_px: i32, // plot's X-axis start (from root's left edge)
) -> Result<()> {
    legend_area
        .fill(&WHITE)
        .map_err(|e| anyhow::anyhow!("{:?}", e))?;

    let (w_u32, _) = legend_area.dim_in_pixel();
    let w = w_u32 as i32;

    // Layout constants (must match estimator)
    let font_px: u32 = 14;
    let line_h: i32 = font_px as i32 + 2;
    let row_gap: i32 = 4;
    let pad_small: i32 = 6;
    let pad_band: i32 = 8;
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

    match placement {
        LegendMode::Right => {
            // Right-panel: simple single-column list (unchanged except error mapping and tighter spacing)
            let pad_x: i32 = 6;

            let mut y = if has_title {
                let title_y_top = pad_small;
                legend_area
                    .draw(&Text::new(title, (pad_x, title_y_top), title_style.clone()))
                    .map_err(|e| anyhow::anyhow!("{:?}", e))?;
                title_y_top + title_font_px as i32 + 8
            } else {
                pad_small + 6
            };

            let text_x = pad_x + 24;
            let max_text_w = (w - text_x - pad_x).max(40) as u32;

            for (label, color) in items {
                let lines = wrap_text_to_width(label, font_px, max_text_w);
                let block_h = (lines.len().max(1) as i32) * line_h;

                let marker_x = pad_x + 12;
                let block_center_y = y + block_h / 2;

                legend_area
                    .draw(&Circle::new(
                        (marker_x, block_center_y),
                        marker_radius,
                        color.clone().filled(),
                    ))
                    .map_err(|e| anyhow::anyhow!("{:?}", e))?;

                for (i, line) in lines.iter().enumerate() {
                    let line_center_y = y + (i as i32) * line_h + line_h / 2;
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
            // Title and top offset
            let start_x = axis_x_start_px;
            let mut y_top = if has_title {
                let title_y_top = pad_band;
                legend_area
                    .draw(&Text::new(
                        title,
                        (start_x, title_y_top),
                        title_style.clone(),
                    ))
                    .map_err(|e| anyhow::anyhow!("{:?}", e))?;
                title_y_top + title_font_px as i32 + 8
            } else {
                pad_band + 8
            };

            // Pass 1: Greedy pack into rows to determine how many columns (K) and row membership
            #[derive(Clone)]
            struct ItemRef {
                label: String,
                color: RGBAColor,
            }
            let usable_row_w = w - pad_small;
            let per_item_cap_px: i32 = ((usable_row_w - start_x) as f32 * 0.35).max(140.0) as i32;

            let mut rows: Vec<Vec<ItemRef>> = Vec::new();
            let mut cur: Vec<ItemRef> = Vec::new();
            let mut x = start_x;

            // helper: packing width with a given cap
            let pack_block_width_for = |label: &str, cap_px: i32| -> i32 {
                let cap = cap_px.max(40) as u32;
                let lines = wrap_text_to_width(label, font_px, cap);
                let max_line_w = lines
                    .iter()
                    .map(|s| estimate_text_width_px(s, font_px) as i32)
                    .max()
                    .unwrap_or(0);
                marker_to_text_gap + marker_radius + max_line_w + trailing_gap
            };

            for (label, color) in items.iter() {
                let remaining_line_px = (usable_row_w - x).max(40);
                let text_cap_now =
                    remaining_line_px - (marker_to_text_gap + marker_radius + trailing_gap);
                let text_cap_now = text_cap_now.min(per_item_cap_px);

                let mut block_w = pack_block_width_for(label, text_cap_now);

                if x + block_w > usable_row_w && !cur.is_empty() {
                    rows.push(cur);
                    cur = Vec::new();
                    x = start_x;

                    let fresh_text_cap = ((usable_row_w - start_x)
                        - (marker_to_text_gap + marker_radius + trailing_gap))
                        .min(per_item_cap_px);
                    block_w = pack_block_width_for(label, fresh_text_cap);
                }

                x += block_w;
                cur.push(ItemRef {
                    label: label.clone(),
                    color: *color,
                });
            }
            if !cur.is_empty() {
                rows.push(cur);
            }

            // Determine K
            let k_cols = rows.iter().map(|r| r.len()).max().unwrap_or(1);

            // Compute per-column preferred block widths from longest single-line label in that column.
            let mut col_block_w: Vec<i32> = vec![60; k_cols];
            for row in rows.iter() {
                for (ci, it) in row.iter().enumerate() {
                    let text_w = estimate_text_width_px(&it.label, font_px) as i32;
                    let block_w = marker_to_text_gap + marker_radius + text_w + trailing_gap;
                    if block_w > col_block_w[ci] {
                        col_block_w[ci] = block_w;
                    }
                }
            }

            // If they fit, use them; else fall back to uniform slots.
            let total_needed = start_x + col_block_w.iter().sum::<i32>();
            let slot_w_per_col: Vec<i32> = if total_needed <= usable_row_w {
                col_block_w.clone()
            } else {
                let uniform = ((usable_row_w - start_x) / (k_cols as i32)).max(60);
                vec![uniform; k_cols]
            };

            // Column x offsets = cumulative sum of slot widths
            let mut col_x: Vec<i32> = Vec::with_capacity(k_cols);
            {
                let mut acc = start_x;
                for sw in slot_w_per_col.iter() {
                    col_x.push(acc);
                    acc += *sw;
                }
            }

            // Per-column text caps (slot minus marker+gap+trailing)
            let text_cap_per_col: Vec<i32> = slot_w_per_col
                .iter()
                .map(|sw| (*sw - (marker_to_text_gap + marker_radius + trailing_gap)).max(40))
                .collect();

            // Render rows using per-column widths/caps
            for row in rows.iter() {
                // compute row height
                let mut row_max_h = line_h;
                let mut blocks_lines: Vec<Vec<String>> = Vec::new();
                for (ci, it) in row.iter().enumerate() {
                    let cap = text_cap_per_col[ci] as u32;
                    let lines = wrap_text_to_width(&it.label, font_px, cap);
                    row_max_h = row_max_h.max((lines.len().max(1) as i32) * line_h);
                    blocks_lines.push(lines);
                }

                let y_center = y_top + row_max_h / 2;

                for (ci, it) in row.iter().enumerate() {
                    let text_x = col_x[ci];
                    let dot_x = (text_x - marker_to_text_gap).max(0);

                    legend_area
                        .draw(&Circle::new(
                            (dot_x, y_center),
                            marker_radius,
                            it.color.clone().filled(),
                        ))
                        .map_err(|e| anyhow::anyhow!("{:?}", e))?;

                    let lines = &blocks_lines[ci];
                    let block_h = (lines.len().max(1) as i32) * line_h;
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
                }

                y_top += row_max_h + row_gap;
            }
        }

        LegendMode::Inside => {
            // Not used for external panel layout
        }
    }

    Ok(())
}

/// Enhanced legend panel that supports line dash patterns and marker shapes.
/// This is used when country-styles mode is enabled with symbols.
pub fn draw_enhanced_legend_panel<DB: DrawingBackend>(
    legend_area: &DrawingArea<DB, Shift>,
    items: &[(String, RGBAColor, Option<MarkerShape>, Option<LineDash>)],
    title: &str, // pass "" to omit (recommended)
    placement: LegendMode,
    axis_x_start_px: i32, // plot's X-axis start (from root's left edge)
) -> Result<()> {
    legend_area
        .fill(&WHITE)
        .map_err(|e| anyhow::anyhow!("{:?}", e))?;

    let (w_u32, _) = legend_area.dim_in_pixel();
    let w = w_u32 as i32;

    // Layout constants (must match estimator)
    let font_px: u32 = 14;
    let line_h: i32 = font_px as i32 + 2;
    let row_gap: i32 = 4;
    let pad_small: i32 = 6;
    let _pad_band: i32 = 8;
    let marker_radius: i32 = 4;
    let marker_to_text_gap: i32 = 12;
    let _trailing_gap: i32 = 12;
    let line_sample_width: i32 = 16; // Width of line sample in legend

    // Styles
    let has_title = !title.trim().is_empty();
    let title_font_px: u32 = 16;
    let title_style: TextStyle = TextStyle::from((FontFamily::SansSerif, title_font_px))
        .pos(Pos::new(HPos::Left, VPos::Top));
    let label_style_center: TextStyle =
        TextStyle::from((FontFamily::SansSerif, font_px)).pos(Pos::new(HPos::Left, VPos::Center));

    match placement {
        LegendMode::Right => {
            // Right-panel: simple single-column list with enhanced glyphs
            let pad_x: i32 = 6;

            let mut y = if has_title {
                let title_y_top = pad_small;
                legend_area
                    .draw(&Text::new(title, (pad_x, title_y_top), title_style.clone()))
                    .map_err(|e| anyhow::anyhow!("{:?}", e))?;
                title_y_top + title_font_px as i32 + 8
            } else {
                pad_small + 6
            };

            let glyph_start_x = pad_x;
            let glyph_width = line_sample_width + marker_radius * 2;
            let text_x = glyph_start_x + glyph_width + marker_to_text_gap;
            let max_text_w = (w - text_x - pad_x).max(40) as u32;

            for (label, color, marker_shape, line_dash) in items {
                let lines = wrap_text_to_width(label, font_px, max_text_w);
                let block_h = (lines.len().max(1) as i32) * line_h;
                let block_center_y = y + block_h / 2;

                // Draw line sample with dash pattern
                let line_start_x = glyph_start_x;
                let line_end_x = glyph_start_x + line_sample_width;
                let line_y = block_center_y;

                draw_legend_line_sample(
                    legend_area,
                    line_start_x,
                    line_end_x,
                    line_y,
                    *color,
                    line_dash.unwrap_or(LineDash::Solid),
                )?;

                // Draw marker shape at center of line
                let marker_x = glyph_start_x + line_sample_width / 2;
                draw_legend_marker(
                    legend_area,
                    marker_x,
                    line_y,
                    marker_radius,
                    *color,
                    marker_shape.unwrap_or(MarkerShape::Circle),
                )?;

                for (i, line) in lines.iter().enumerate() {
                    let line_center_y = y + (i as i32) * line_h + line_h / 2;
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
            // For now, fall back to simple circles for top/bottom legends
            // This can be enhanced later with proper glyph rendering
            let simple_items: Vec<(String, RGBAColor)> = items
                .iter()
                .map(|(label, color, _, _)| (label.clone(), *color))
                .collect();

            return draw_legend_panel(
                legend_area,
                &simple_items,
                title,
                placement,
                axis_x_start_px,
            );
        }

        LegendMode::Inside => {
            // Not used for external panel layout
        }
    }

    Ok(())
}

/// Draw a line sample with the specified dash pattern
fn draw_legend_line_sample<DB: DrawingBackend>(
    legend_area: &DrawingArea<DB, Shift>,
    start_x: i32,
    end_x: i32,
    y: i32,
    color: RGBAColor,
    line_dash: LineDash,
) -> Result<()> {
    let line_style = ShapeStyle {
        color,
        filled: false,
        stroke_width: 2,
    };

    match line_dash {
        LineDash::Solid => {
            legend_area
                .draw(&PathElement::new(
                    vec![(start_x, y), (end_x, y)],
                    line_style,
                ))
                .map_err(|e| anyhow::anyhow!("{:?}", e))?;
        }
        LineDash::Dash => {
            // Draw dashed line sample
            let segment_len = 4;
            let gap_len = 3;
            let mut x = start_x;
            while x < end_x {
                let segment_end = (x + segment_len).min(end_x);
                legend_area
                    .draw(&PathElement::new(
                        vec![(x, y), (segment_end, y)],
                        line_style,
                    ))
                    .map_err(|e| anyhow::anyhow!("{:?}", e))?;
                x = segment_end + gap_len;
            }
        }
        LineDash::Dot => {
            // Draw dotted line sample
            let dot_spacing = 3;
            let mut x = start_x;
            while x <= end_x {
                legend_area
                    .draw(&Circle::new((x, y), 1, color.filled()))
                    .map_err(|e| anyhow::anyhow!("{:?}", e))?;
                x += dot_spacing;
            }
        }
        LineDash::DashDot => {
            // Draw dash-dot pattern sample
            let dash_len = 6;
            let dot_gap = 2;
            let mut x = start_x;
            let mut is_dash = true;
            while x < end_x {
                if is_dash {
                    let segment_end = (x + dash_len).min(end_x);
                    legend_area
                        .draw(&PathElement::new(
                            vec![(x, y), (segment_end, y)],
                            line_style,
                        ))
                        .map_err(|e| anyhow::anyhow!("{:?}", e))?;
                    x = segment_end + dot_gap;
                } else {
                    legend_area
                        .draw(&Circle::new((x, y), 1, color.filled()))
                        .map_err(|e| anyhow::anyhow!("{:?}", e))?;
                    x += 1 + dot_gap;
                }
                is_dash = !is_dash;
            }
        }
    }

    Ok(())
}

/// Draw a marker in the legend with the specified shape
fn draw_legend_marker<DB: DrawingBackend>(
    legend_area: &DrawingArea<DB, Shift>,
    x: i32,
    y: i32,
    size: i32,
    color: RGBAColor,
    marker_shape: MarkerShape,
) -> Result<()> {
    match marker_shape {
        MarkerShape::Circle => {
            legend_area
                .draw(&Circle::new((x, y), size, color.filled()))
                .map_err(|e| anyhow::anyhow!("{:?}", e))?;
        }
        MarkerShape::Square => {
            legend_area
                .draw(&Rectangle::new(
                    [(x - size, y - size), (x + size, y + size)],
                    color.filled(),
                ))
                .map_err(|e| anyhow::anyhow!("{:?}", e))?;
        }
        MarkerShape::Triangle => {
            legend_area
                .draw(&Polygon::new(
                    vec![(x, y - size), (x - size, y + size), (x + size, y + size)],
                    color.filled(),
                ))
                .map_err(|e| anyhow::anyhow!("{:?}", e))?;
        }
        MarkerShape::Diamond => {
            legend_area
                .draw(&Polygon::new(
                    vec![(x, y - size), (x - size, y), (x, y + size), (x + size, y)],
                    color.filled(),
                ))
                .map_err(|e| anyhow::anyhow!("{:?}", e))?;
        }
        MarkerShape::Cross => {
            let line_style = ShapeStyle {
                color,
                filled: false,
                stroke_width: 2,
            };
            legend_area
                .draw(&PathElement::new(
                    vec![(x - size, y), (x + size, y)],
                    line_style,
                ))
                .map_err(|e| anyhow::anyhow!("{:?}", e))?;
            legend_area
                .draw(&PathElement::new(
                    vec![(x, y - size), (x, y + size)],
                    line_style,
                ))
                .map_err(|e| anyhow::anyhow!("{:?}", e))?;
        }
        MarkerShape::X => {
            let line_style = ShapeStyle {
                color,
                filled: false,
                stroke_width: 2,
            };
            legend_area
                .draw(&PathElement::new(
                    vec![(x - size, y - size), (x + size, y + size)],
                    line_style,
                ))
                .map_err(|e| anyhow::anyhow!("{:?}", e))?;
            legend_area
                .draw(&PathElement::new(
                    vec![(x - size, y + size), (x + size, y - size)],
                    line_style,
                ))
                .map_err(|e| anyhow::anyhow!("{:?}", e))?;
        }
    }

    Ok(())
}
