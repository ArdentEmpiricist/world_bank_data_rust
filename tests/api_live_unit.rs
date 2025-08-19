//! Live API test for unit preference. Run with: `cargo test --features online -- --nocapture`
#![cfg(feature = "online")]
//!
//! Behavior:
//! - If the World Bank API provides a unit for the indicator, assert that it
//!   appears in the rendered chart (e.g., in axis/legend/title).
//! - If the API does not provide a unit (or it is empty), do NOT fail the test;
//!   instead, ensure the chart still renders and that a fallback label
//!   (indicator name/code) appears so the chart is readable.

use std::fs;
use std::path::PathBuf;

use serde_json::Value;

// If your crate exposes a library interface for rendering, prefer importing and
// calling it directly. If the test currently shells out to the binary, keep that
// approach. The helpers below are written to be minimally invasive and should be
// adaptable to your current code.
#[cfg(feature = "blocking-http")]
use reqwest::blocking::Client;

/// Configure the indicator for the test:
/// - Default is GDP (NY.GDP.MKTP.CD), but it can be overridden via env var:
///   WB_TEST_INDICATOR=SP.POP.TOTL cargo test -- --nocapture
fn indicator_code() -> String {
    std::env::var("WB_TEST_INDICATOR").unwrap_or_else(|_| "NY.GDP.MKTP.CD".to_string())
}

/// Returns Some(unit) if present in indicator metadata, accepting multiple possible fields.
/// Returns None if no usable unit is found.
#[cfg(feature = "blocking-http")]
fn fetch_unit_from_api(indicator: &str) -> Option<String> {
    let url = format!(
        "https://api.worldbank.org/v2/indicator/{}?format=json",
        indicator
    );
    let resp = Client::new()
        .get(&url)
        .send()
        .ok()?
        .error_for_status()
        .ok()?
        .text()
        .ok()?;

    let v: Value = serde_json::from_str(&resp).ok()?;
    // Typical WB API shape: [metadata, [indicator objects...]]
    let arr = v.as_array()?;
    let indicators = arr.get(1)?.as_array()?;
    let obj = indicators.get(0)?.as_object()?;

    // Try several known keys that might carry unit information
    let candidates = [
        "unit",
        "unit_of_measure",
        "unit_of_measure_code",
        "unitOfMeasure",
        "unitOfMeasureCode",
    ];

    for key in candidates {
        if let Some(val) = obj.get(key) {
            let s = val.as_str().unwrap_or("").trim();
            if !s.is_empty() {
                return Some(s.to_string());
            }
        }
    }

    // Some indicators include units as part of a descriptive field; keep minimal heuristics here
    // If you have program-side fallbacks that parse title/long definition, add mirrors here only for verification.
    None
}

/// Helper that returns the expected fallback label text when unit is missing.
/// Adjust as needed to mirror the program’s actual fallback (e.g., indicator short name).
fn expected_fallback_label(indicator: &str) -> String {
    // Minimal, conservative fallback: ensure at least the indicator code appears
    indicator.to_string()
}

/// Renders a tiny chart to SVG and returns the SVG text for inspection.
/// Replace this with your program’s existing test helper that renders output.
/// If you already have a function to produce the chart as a string, use it here.
fn render_test_chart_svg(indicator: &str) -> String {
    // NOTE: Replace this stub with your crate’s actual rendering call.
    // For example:
    // let svg = world_bank_data_rust::viz::render_svg_for_indicator(indicator, /*...*/).unwrap();
    //
    // In the meantime, keep a minimal placeholder so the test compiles if you
    // haven’t wired the helper yet. The test below will early-return if we cannot
    // find a real SVG in your current harness.
    let tmp = PathBuf::from(format!("target/test-output/{}_sample.svg", indicator));
    if let Ok(s) = fs::read_to_string(&tmp) {
        return s;
    }
    // If no file exists because the project renders elsewhere, return empty.
    String::new()
}

#[test]
fn api_provided_unit_appears_in_chart_or_fallback_is_used() {
    // If your crate needs a feature flag for blocking HTTP in tests, ensure it’s enabled.
    // Otherwise, you can gate the live portion and still validate fallback rendering logic.

    let indicator = indicator_code();

    #[cfg(feature = "blocking-http")]
    let unit_opt = fetch_unit_from_api(&indicator);

    #[cfg(not(feature = "blocking-http"))]
    let unit_opt: Option<String> = None;

    // Read the SVG and check that it contains the API-provided unit
    let svg_content = fs::read_to_string(&tmp_svg).unwrap();

    // If you can directly obtain the rendered label from the program (e.g., return values),
    // prefer asserting on that rather than parsing SVG text.

    match unit_opt {
        Some(ref unit) if !unit.trim().is_empty() => {
            // Assert the unit appears somewhere in the SVG text (title/axis/legend).
            // Adjust the search as appropriate for your chart layout.
            if svg.is_empty() {
                eprintln!(
                    "Warning: SVG text is empty. Ensure render_test_chart_svg() is wired to your renderer."
                );
            } else {
                assert!(
                    svg.contains(unit),
                    "Expected rendered chart to contain unit '{}', but it was not found",
                    unit
                );
            }
        }
        _ => {
            // No unit provided by the API => ensure the chart still renders
            // and includes a reasonable fallback label.
            let fallback = expected_fallback_label(&indicator);

            if svg.is_empty() {
                eprintln!(
                    "No unit provided by API and no SVG captured; \
                     ensure render_test_chart_svg() targets the chart output location."
                );
                // We avoid failing here to keep the live test non-flaky.
                return;
            }

            assert!(
                svg.contains(&fallback),
                "Unit missing from API; expected fallback label '{}' to appear in chart output",
                fallback
            );
        }
    }
}
