//! Styling utilities to consistently map (country, indicator) to colors, shapes, and line styles.
//!
//! Design:
//! - Country: assigned a stable base hue (primary identity).
//! - Indicator: encodes variation as a shade/saturation offset and a marker/line-dash style (redundant).
//!
//! All comments and docs are in English.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[derive(Clone, Copy, Debug)]
pub enum MarkerShape {
    Circle,
    Square,
    Triangle,
    Diamond,
    Cross,
    X,
}

#[derive(Clone, Copy, Debug)]
pub enum LineDash {
    Solid,
    Dash,
    Dot,
    DashDot,
}

#[derive(Clone, Copy, Debug)]
pub struct Rgb8 {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Clone, Copy, Debug)]
pub struct Hsl {
    pub h_deg: f64, // 0..360
    pub s: f64,     // 0..1
    pub l: f64,     // 0..1
}

#[derive(Clone, Debug)]
pub struct SeriesStyle {
    pub country: String,
    pub indicator: String,
    pub hsl: Hsl,
    pub rgb: Rgb8,
    pub hex: String,
    pub marker: MarkerShape,
    pub line_dash: LineDash,
    pub marker_size: u32,
    pub line_width: u32,
}

impl SeriesStyle {
    /// Build a consistent style for a (country, indicator) pair.
    pub fn for_series(country: &str, indicator: &str) -> Self {
        let base_hue = stable_hue_deg(country);
        let base = Hsl {
            h_deg: base_hue,
            s: 0.60,
            l: 0.55,
        };

        // Indicator variation: deterministic lightness and slight saturation offset.
        let (dl, ds) = indicator_offsets(indicator);
        let varied = Hsl {
            h_deg: base.h_deg,
            s: clamp01(base.s + ds),
            l: clamp01(base.l + dl),
        };

        let rgb = hsl_to_rgb8(varied);
        let hex = rgb_to_hex(rgb);

        let marker = indicator_to_marker(indicator);
        let line_dash = indicator_to_dash(indicator);

        SeriesStyle {
            country: country.to_string(),
            indicator: indicator.to_string(),
            hsl: varied,
            rgb,
            hex,
            marker,
            line_dash,
            marker_size: 6,
            line_width: 2,
        }
    }
}

// ------------------------ Mapping logic ------------------------

fn stable_hue_deg(key: &str) -> f64 {
    // Hash to 0..359 for a hue angle
    let h = stable_hash64(key);
    (h % 360) as f64
}

fn indicator_offsets(indicator: &str) -> (f64, f64) {
    // Map indicator to (delta_lightness, delta_saturation)
    // Lightness in [-0.18, +0.18], Saturation in [-0.10, +0.10]
    let h = stable_hash64(indicator);
    let dl = map_u64_to_range(h.rotate_left(13), -0.18, 0.18);
    let ds = map_u64_to_range(h.rotate_left(29), -0.10, 0.10);
    (dl, ds)
}

fn indicator_to_marker(indicator: &str) -> MarkerShape {
    match (stable_hash64(indicator) % 6) as u8 {
        0 => MarkerShape::Circle,
        1 => MarkerShape::Square,
        2 => MarkerShape::Triangle,
        3 => MarkerShape::Diamond,
        4 => MarkerShape::Cross,
        _ => MarkerShape::X,
    }
}

fn indicator_to_dash(indicator: &str) -> LineDash {
    match (stable_hash64(indicator) % 4) as u8 {
        0 => LineDash::Solid,
        1 => LineDash::Dash,
        2 => LineDash::Dot,
        _ => LineDash::DashDot,
    }
}

// ------------------------ Utilities ------------------------

fn stable_hash64<T: Hash>(t: T) -> u64 {
    let mut hasher = DefaultHasher::new();
    t.hash(&mut hasher);
    hasher.finish()
}

fn clamp01(x: f64) -> f64 {
    x.clamp(0.0, 1.0)
}

fn map_u64_to_range(x: u64, min: f64, max: f64) -> f64 {
    let t = (x as f64) / (u64::MAX as f64); // 0..1
    min + t * (max - min)
}

// HSL -> RGB conversion (linear; sufficient for chart colors)
fn hsl_to_rgb8(hsl: Hsl) -> Rgb8 {
    let h = (hsl.h_deg % 360.0) / 360.0;
    let s = clamp01(hsl.s);
    let l = clamp01(hsl.l);

    if s == 0.0 {
        let v = (l * 255.0).round() as u8;
        return Rgb8 { r: v, g: v, b: v };
    }

    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;

    fn hue_to_rgb(p: f64, q: f64, mut t: f64) -> f64 {
        if t < 0.0 {
            t += 1.0;
        }
        if t > 1.0 {
            t -= 1.0;
        }
        if t < 1.0 / 6.0 {
            p + (q - p) * 6.0 * t
        } else if t < 1.0 / 2.0 {
            q
        } else if t < 2.0 / 3.0 {
            p + (q - p) * (2.0 / 3.0 - t) * 6.0
        } else {
            p
        }
    }

    let r = hue_to_rgb(p, q, h + 1.0 / 3.0);
    let g = hue_to_rgb(p, q, h);
    let b = hue_to_rgb(p, q, h - 1.0 / 3.0);

    Rgb8 {
        r: (r * 255.0).round() as u8,
        g: (g * 255.0).round() as u8,
        b: (b * 255.0).round() as u8,
    }
}

fn rgb_to_hex(rgb: Rgb8) -> String {
    format!("#{:02X}{:02X}{:02X}", rgb.r, rgb.g, rgb.b)
}
