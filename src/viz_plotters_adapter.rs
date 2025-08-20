//! Adapter helpers to use SeriesStyle with the plotters crate.
//!
//! Usage example (inside your plotting function):
//! ```ignore
//!     use plotters::prelude::*;
//!     use crate::viz_style::SeriesStyle;
//!     use crate::viz_plotters_adapter::{rgb_color, line_style, fill_style, make_marker};
//!
//!     // For each (country, indicator) series:
//!     let style = SeriesStyle::for_series(country_code, indicator_code);
//!
//!     // 1) Draw a line (optional):
//!     let stroke = line_style(&style);
//!     chart.draw_series(LineSeries::new(series_points.clone(), stroke))?
//!         .label(format!("{} — {}", country_code, indicator_code));
//!
//!     // 2) Draw markers along the line (recommended for indicator redundancy):
//!     chart.draw_series(PointSeries::of_element(
//!         series_points,                                  // iterator of (x, y) in data coords
//!         style.marker_size as i32,                      // marker size
//!         fill_style(&style),                            // pass ShapeStyle so closure gets &ShapeStyle
//!         &|c, s, st| make_marker::<BitMapBackend>(*c, s, st.clone(), style.marker),
//!     ))?;
//!
//!     // 3) For bars: use `fill_style(&style)` with Rectangle::new(..., fill_style(&style))
//!
//! All comments and docs are in English.

use plotters::element::DynElement;
use plotters::prelude::*;

use crate::viz_style::{LineDash, MarkerShape, SeriesStyle};

pub fn rgb_color(style: &SeriesStyle) -> RGBColor {
    RGBColor(style.rgb.r, style.rgb.g, style.rgb.b)
}

/// Build a ShapeStyle for line strokes.
/// Plotters’ dashed strokes are backend-dependent; combine lines with markers for redundancy.
pub fn line_style(style: &SeriesStyle) -> ShapeStyle {
    rgb_color(style).stroke_width(style.line_width)
}

/// Get the dash pattern for a given line dash style.
/// Returns None for solid lines, Some(pattern) for dashed lines.
/// Lengths are scaled by line width as specified in the requirements.
pub fn dash_pattern(dash: LineDash, line_width: u32) -> Option<Vec<i32>> {
    let lw = line_width as i32;
    match dash {
        LineDash::Solid => None,
        LineDash::Dash => Some(vec![6 * lw, 4 * lw]), // on ≈ 6×line_width px, off ≈ 4×line_width px
        LineDash::Dot => Some(vec![2 * lw, 4 * lw]),  // on ≈ 2×line_width px, off ≈ 4×line_width px  
        LineDash::DashDot => Some(vec![6 * lw, 3 * lw, 2 * lw, 3 * lw]), // on ≈ 6×line_width px, off ≈ 3×line_width px, on ≈ 2×line_width px, off ≈ 3×line_width px
    }
}

/// Create an iterator of marker elements for the given points and marker shape.
/// This provides a simple way to render different marker shapes.
pub fn create_marker_elements(
    points: &[(f64, f64)],
    size: i32,
    color: RGBAColor,
    marker: MarkerShape,
) -> Vec<Box<dyn Fn() -> Circle<(f64, f64), i32> + '_>> {
    // For now, simplify to just use circles but with different sizes per marker type
    // This is a stepping stone toward full marker shape support
    let marker_size = match marker {
        MarkerShape::Circle => size,
        MarkerShape::Square => size + 1,
        MarkerShape::Triangle => size + 1,
        MarkerShape::Diamond => size + 1,
        MarkerShape::Cross => size + 2,
        MarkerShape::X => size + 2,
    };
    
    points.iter().map(move |(x, y)| {
        Box::new(move || Circle::new((*x, *y), marker_size, color.filled()))
            as Box<dyn Fn() -> Circle<(f64, f64), i32>>
    }).collect()
}

/// Build a filled style for bars (or simple filled shapes).
pub fn fill_style(style: &SeriesStyle) -> ShapeStyle {
    rgb_color(style).filled()
}

/// Draw a compact legend swatch that represents both the line and the marker.
pub fn legend_swatch<DB>(
    x: i32,
    y: i32,
    style: &SeriesStyle,
    marker: MarkerShape,
) -> DynElement<'static, DB, (i32, i32)>
where
    DB: DrawingBackend + 'static,
{
    let st = line_style(style);
    let marker_size = style.marker_size as i32;
    (EmptyElement::at((x, y))
        + PathElement::new(vec![(x - 14, y), (x + 14, y)], st.clone())
        + make_marker::<DB>((x, y), marker_size, fill_style(style), marker))
    .into_dyn()
}
/// This version uses the concrete coordinate type `(i32, i32)` and requires the backend `DB` to be `'static`.
///
/// Call from PointSeries::of_element with:
///   &|c, s, st| make_marker::<BitMapBackend>(*c, s, st.clone(), style.marker)
pub fn make_marker<DB>(
    c: (i32, i32),
    s: i32,
    st: ShapeStyle,
    marker: MarkerShape,
) -> DynElement<'static, DB, (i32, i32)>
where
    DB: DrawingBackend + 'static,
{
    match marker {
        MarkerShape::Circle => {
            (EmptyElement::at(c) + Circle::new((0, 0), s, st.filled())).into_dyn()
        }
        MarkerShape::Square => {
            (EmptyElement::at(c) + Rectangle::new([(-s, -s), (s, s)], st.filled())).into_dyn()
        }
        MarkerShape::Triangle => (EmptyElement::at(c)
            + Polygon::new(vec![(0, -s), (-s, s), (s, s)], st.filled()))
        .into_dyn(),
        MarkerShape::Diamond => (EmptyElement::at(c)
            + Polygon::new(vec![(0, -s), (-s, 0), (0, s), (s, 0)], st.filled()))
        .into_dyn(),
        MarkerShape::Cross => (EmptyElement::at(c)
            + PathElement::new(vec![(-s, 0), (s, 0)], st.stroke_width(2))
            + PathElement::new(vec![(0, -s), (0, s)], st.stroke_width(2)))
        .into_dyn(),
        MarkerShape::X => (EmptyElement::at(c)
            + PathElement::new(vec![(-s, -s), (s, s)], st.stroke_width(2))
            + PathElement::new(vec![(-s, s), (s, -s)], st.stroke_width(2)))
        .into_dyn(),
    }
}
