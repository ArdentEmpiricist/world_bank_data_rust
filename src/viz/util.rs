//! Utility functions for visualization: colors, scaling, locale mapping, unit detection.

use crate::models::DataPoint;
use num_format::Locale;
use plotters::prelude::*;
use std::collections::BTreeSet;

use super::text::estimate_text_width_px;

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

/// Get a color from the Office palette.
#[inline]
pub fn office_color(idx: usize) -> RGBAColor {
    OFFICE10[idx % OFFICE10.len()].to_rgba()
}

/// Pick a single Y-axis scale and its human label based on the overall magnitude.
/// Returns (scale, label), e.g. (1e6, "millions").
pub fn choose_axis_scale(max_abs: f64) -> (f64, &'static str) {
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
pub fn extract_unit_from_indicator_name(name: &str) -> Option<String> {
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

/// Derive a common unit string, preferring API-provided units when consistent.
///
/// Logic:
/// 1) Collect all non-empty `unit` values from DataPoint.unit
/// 2) If there's exactly one unique non-empty unit string, use it
/// 3) Otherwise, fall back to existing behavior: if single indicator, extract unit from name
pub fn derive_axis_unit(points: &[DataPoint]) -> Option<String> {
    // 1) Prefer consistent API-provided unit
    let units: BTreeSet<&str> = points
        .iter()
        .filter_map(|p| p.unit.as_deref())
        .filter(|u| !u.is_empty())
        .collect();

    if units.len() == 1 {
        return Some(units.iter().next().unwrap().to_string());
    }

    // 2) Fall back to existing behavior: extract from indicator name if single indicator
    let names: BTreeSet<&str> = points.iter().map(|p| p.indicator_name.as_str()).collect();
    if names.len() == 1 {
        extract_unit_from_indicator_name(names.iter().next().unwrap())
    } else {
        None
    }
}

/// Heuristic: treat percent-like units as non-scalable (no thousands/millions/billions).
pub fn is_percentage_like(unit: &str) -> bool {
    let u = unit.to_ascii_lowercase();
    u.contains('%') || u.contains("percent") || u.contains("percentage") || u.contains("per cent")
}

/// Map a user-provided locale tag to a `num_format::Locale` and its decimal separator char.
///
/// Supported tags (case-insensitive): `en`, `us`, `en_US`, `de`, `de_DE`, `german`,
/// `fr`, `es`, `it`, `pt`, `nl`. Defaults to English.
pub fn map_locale(tag: &str) -> (&'static Locale, char) {
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

/// Compute a tight left label area width for the Y axis (in pixels),
/// based on the formatted tick labels that will appear.
/// - `ymin_scaled..ymax_scaled`: the **scaled** Y range you pass to Plotters
/// - `ticks`: how many Y labels you plan to show (e.g., 10)
/// - `font_px`: font size used for axis labels (e.g., 12)
///
/// Returns a width clamped to a sensible range to avoid extremes.
pub fn compute_left_label_area_px(
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
