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

use plotters::element::DynElement;
use plotters::prelude::*;

use crate::viz_style::{MarkerShape, SeriesStyle};

pub fn rgb_color(style: &SeriesStyle) -> RGBColor {
    RGBColor(style.rgb.r, style.rgb.g, style.rgb.b)
}

/// Build a ShapeStyle for line strokes.
/// Plotters’ dashed strokes are backend-dependent; combine lines with markers for redundancy.
pub fn line_style(style: &SeriesStyle) -> ShapeStyle {
    rgb_color(style).stroke_width(style.line_width)
}

/// Build a filled style for bars (or simple filled shapes).
pub fn fill_style(style: &SeriesStyle) -> ShapeStyle {
    rgb_color(style).filled()
}

/// Construct a marker DynElement at the given anchor coordinate `c`.
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
