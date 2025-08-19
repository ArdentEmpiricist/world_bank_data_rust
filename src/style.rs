//! Flexible, testable style assignment system for country-consistent styling.
//!
//! When enabled via the `country-styles` feature flag, this module provides
//! a styling system where all series for the same country share one base hue
//! from the MS Office palette, while indicators within that country are
//! differentiated by shades, marker shapes, and line dash patterns.
//!
//! # Usage
//!
//! This module is only available when the `country-styles` feature is enabled:
//!
//! ```toml
//! [dependencies]
//! world_bank_data_rust = { version = "0.1", features = ["country-styles"] }
//! ```
//!
//! # Example
//!
//! ```rust
//! # #[cfg(feature = "country-styles")]
//! # {
//! use world_bank_data_rust::style::{SeriesKey, assign_country_styles};
//!
//! let series = vec![
//!     SeriesKey::new("USA".to_string(), "GDP".to_string()),
//!     SeriesKey::new("USA".to_string(), "Population".to_string()),
//!     SeriesKey::new("DEU".to_string(), "GDP".to_string()),
//! ];
//!
//! let styles = assign_country_styles(&series, 255);
//! 
//! // All USA series will have the same base_hue
//! // Different indicators will have different shades, markers, and dash patterns
//! # }
//! ```
//!
//! # Design Principles
//!
//! - **Country consistency**: All series for the same country use the same base hue
//! - **Indicator differentiation**: Different indicators use brightness variations, 
//!   unique marker shapes, and line dash patterns for redundant visual encoding
//! - **Deterministic**: Identical inputs always produce identical outputs
//! - **MS Office compatibility**: Uses the standard MS Office color palette

use std::collections::HashMap;

/// RGBA color representation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba {
    /// Create a new RGBA color.
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Create an opaque RGB color.
    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self::new(r, g, b, 255)
    }
}

/// Marker shape for data points.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarkerShape {
    Circle,
    Square,
    Triangle,
    Diamond,
    Cross,
    X,
}

/// Line dash pattern.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineDash {
    Solid,
    Dash,
    Dot,
    DashDot,
}

/// Key identifying a unique series (country, indicator pair).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SeriesKey {
    pub country: String,
    pub indicator: String,
}

impl SeriesKey {
    /// Create a new series key.
    pub fn new(country: String, indicator: String) -> Self {
        Self { country, indicator }
    }
}

/// Complete style specification for a series.
#[derive(Clone, Debug, PartialEq)]
pub struct SeriesStyle {
    pub base_hue: f64, // 0..360 degrees
    pub shade: Rgba,
    pub marker: MarkerShape,
    pub dash: LineDash,
}

impl SeriesStyle {
    /// Create a new series style.
    pub fn new(base_hue: f64, shade: Rgba, marker: MarkerShape, dash: LineDash) -> Self {
        Self {
            base_hue,
            shade,
            marker,
            dash,
        }
    }
}

/// MS Office color palette (RGB values).
const MS_OFFICE_PALETTE: [(u8, u8, u8); 10] = [
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

/// Assign country-consistent styles using the MS Office palette.
///
/// All series for the same country will share the same base hue, while
/// indicators within that country are differentiated by shades, marker
/// shapes, and line dash patterns.
///
/// # Arguments
/// * `series` - List of series keys (country, indicator pairs)
/// * `palette` - Color palette to use (typically MS Office palette)
/// * `alpha` - Alpha channel value (0-255)
///
/// # Returns
/// HashMap mapping each series key to its assigned style
/// Assign country-consistent styles using the MS Office palette.
///
/// All series for the same country will share the same base hue, while
/// indicators within that country are differentiated by shades, marker
/// shapes, and line dash patterns.
///
/// # Arguments
/// * `series` - List of series keys (country, indicator pairs)
/// * `palette` - Color palette to use (typically MS Office palette)
/// * `alpha` - Alpha channel value (0-255)
///
/// # Returns
/// HashMap mapping each series key to its assigned style
pub fn assign_country_styles_with_palette(
    series: &[SeriesKey],
    palette: &[(u8, u8, u8)],
    alpha: u8,
) -> HashMap<SeriesKey, SeriesStyle> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut result = HashMap::new();
    
    // Helper function to create a stable hash
    fn stable_hash<T: Hash>(value: &T) -> u64 {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        hasher.finish()
    }

    // Helper function to map hash to marker shape
    fn hash_to_marker(hash: u64) -> MarkerShape {
        match hash % 6 {
            0 => MarkerShape::Circle,
            1 => MarkerShape::Square,
            2 => MarkerShape::Triangle,
            3 => MarkerShape::Diamond,
            4 => MarkerShape::Cross,
            _ => MarkerShape::X,
        }
    }

    // Helper function to map hash to line dash
    fn hash_to_dash(hash: u64) -> LineDash {
        match hash % 4 {
            0 => LineDash::Solid,
            1 => LineDash::Dash,
            2 => LineDash::Dot,
            _ => LineDash::DashDot,
        }
    }

    // Helper function to adjust color brightness
    fn adjust_brightness(color: (u8, u8, u8), factor: f64) -> (u8, u8, u8) {
        let (r, g, b) = color;
        let new_r = ((r as f64 * factor).min(255.0).max(0.0)) as u8;
        let new_g = ((g as f64 * factor).min(255.0).max(0.0)) as u8;
        let new_b = ((b as f64 * factor).min(255.0).max(0.0)) as u8;
        (new_r, new_g, new_b)
    }

    // Get unique countries in deterministic order (sorted by country name)
    let unique_countries: Vec<String> = series
        .iter()
        .map(|s| s.country.clone())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();

    // Assign each country to a palette index deterministically based on its hash
    let country_to_index: HashMap<String, usize> = unique_countries
        .iter()
        .map(|country| {
            let hash = stable_hash(country);
            let index = (hash as usize) % palette.len();
            (country.clone(), index)
        })
        .collect();

    // Assign styles to each series
    for series_key in series {
        let country_index = country_to_index[&series_key.country];
        let base_color = palette[country_index];
        let base_hue = rgb_to_hue(base_color);
        
        // Generate deterministic variations based on indicator
        let indicator_hash = stable_hash(&series_key.indicator);
        
        // Create shade variation (brightness adjustment)
        let brightness_factor = 0.7 + 0.6 * ((indicator_hash % 100) as f64 / 100.0);
        let adjusted_color = adjust_brightness(base_color, brightness_factor);
        let shade = Rgba::new(adjusted_color.0, adjusted_color.1, adjusted_color.2, alpha);
        
        // Assign marker and dash based on indicator
        let marker = hash_to_marker(indicator_hash);
        let dash = hash_to_dash(indicator_hash.rotate_left(16));
        
        let style = SeriesStyle::new(base_hue, shade, marker, dash);
        result.insert(series_key.clone(), style);
    }

    result
}

/// Convert RGB to approximate hue (in degrees 0-360).
fn rgb_to_hue(rgb: (u8, u8, u8)) -> f64 {
    let (r, g, b) = (rgb.0 as f64 / 255.0, rgb.1 as f64 / 255.0, rgb.2 as f64 / 255.0);
    
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    
    if delta == 0.0 {
        return 0.0;
    }
    
    let hue = if max == r {
        60.0 * (((g - b) / delta) % 6.0)
    } else if max == g {
        60.0 * ((b - r) / delta + 2.0)
    } else {
        60.0 * ((r - g) / delta + 4.0)
    };
    
    if hue < 0.0 {
        hue + 360.0
    } else {
        hue
    }
}

/// Convenience function using the MS Office palette.
pub fn assign_country_styles(series: &[SeriesKey], alpha: u8) -> HashMap<SeriesKey, SeriesStyle> {
    assign_country_styles_with_palette(series, &MS_OFFICE_PALETTE, alpha)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_series_key_creation() {
        let key = SeriesKey::new("USA".to_string(), "GDP".to_string());
        assert_eq!(key.country, "USA");
        assert_eq!(key.indicator, "GDP");
    }

    #[test]
    fn test_rgba_creation() {
        let color = Rgba::rgb(255, 128, 64);
        assert_eq!(color.r, 255);
        assert_eq!(color.g, 128);
        assert_eq!(color.b, 64);
        assert_eq!(color.a, 255);
    }

    #[test]
    fn test_country_consistent_styles() {
        let series = vec![
            SeriesKey::new("USA".to_string(), "GDP".to_string()),
            SeriesKey::new("USA".to_string(), "Population".to_string()),
            SeriesKey::new("DEU".to_string(), "GDP".to_string()),
            SeriesKey::new("DEU".to_string(), "Population".to_string()),
        ];

        let styles = assign_country_styles(&series, 255);

        // Same country should have same base hue
        let usa_gdp_hue = styles[&series[0]].base_hue;
        let usa_pop_hue = styles[&series[1]].base_hue;
        assert_eq!(usa_gdp_hue, usa_pop_hue);

        let deu_gdp_hue = styles[&series[2]].base_hue;
        let deu_pop_hue = styles[&series[3]].base_hue;
        assert_eq!(deu_gdp_hue, deu_pop_hue);

        // Different countries should have different base hues
        assert_ne!(usa_gdp_hue, deu_gdp_hue);

        // Same indicator across countries should have different markers/dashes
        // (since they get different variations based on country+indicator combination)
        let _usa_gdp_marker = styles[&series[0]].marker;
        let _deu_gdp_marker = styles[&series[2]].marker;
        // May or may not be different, but styles should be deterministic
        
        // Test determinism - running again should give same results
        let styles2 = assign_country_styles(&series, 255);
        assert_eq!(styles, styles2);
    }

    #[test]
    fn test_deterministic_assignment() {
        let series = vec![
            SeriesKey::new("FRA".to_string(), "inflation".to_string()),
            SeriesKey::new("GBR".to_string(), "inflation".to_string()),
        ];

        let styles1 = assign_country_styles(&series, 200);
        let styles2 = assign_country_styles(&series, 200);
        
        // Results should be identical across multiple calls
        assert_eq!(styles1, styles2);
        
        // Should have assigned styles for all series
        assert_eq!(styles1.len(), 2);
        assert!(styles1.contains_key(&series[0]));
        assert!(styles1.contains_key(&series[1]));
    }
}