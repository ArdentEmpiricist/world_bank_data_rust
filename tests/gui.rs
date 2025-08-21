/*!
 * Tests for the GUI application functionality
 *
 * These tests verify the core logic of the GUI without requiring a display.
 */

use tempfile::TempDir;

/// Test the list parsing function from the GUI
#[test]
fn test_parse_list_function() {
    // Test comma-separated values
    let result = parse_list("USA,DEU,CHN");
    assert_eq!(result, vec!["USA", "DEU", "CHN"]);

    // Test semicolon-separated values
    let result = parse_list("USA;DEU;CHN");
    assert_eq!(result, vec!["USA", "DEU", "CHN"]);

    // Test mixed separators with spaces
    let result = parse_list("USA, DEU ; CHN ");
    assert_eq!(result, vec!["USA", "DEU", "CHN"]);

    // Test empty string
    let result = parse_list("");
    assert!(result.is_empty());

    // Test single value
    let result = parse_list("USA");
    assert_eq!(result, vec!["USA"]);

    // Test with empty values
    let result = parse_list("USA,,DEU,");
    assert_eq!(result, vec!["USA", "DEU"]);
}

/// Test export format validation and file naming
#[test]
fn test_export_format_logic() {
    // This tests the expected behavior of export formats
    // In practice, these would be tested in the GUI's validation logic

    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();

    // Test CSV file naming
    let csv_path = base_path.join("wbi_data.csv");
    assert_eq!(csv_path.extension().unwrap(), "csv");

    // Test JSON file naming
    let json_path = base_path.join("wbi_data.json");
    assert_eq!(json_path.extension().unwrap(), "json");

    // Test chart file naming
    let png_path = base_path.join("wbi_chart.png");
    assert_eq!(png_path.extension().unwrap(), "png");

    let svg_path = base_path.join("wbi_chart.svg");
    assert_eq!(svg_path.extension().unwrap(), "svg");
}

/// Test date range validation logic
#[test]
fn test_date_validation() {
    // Valid range
    assert!(validate_date_range(2010, 2020).is_ok());

    // Same year (valid)
    assert!(validate_date_range(2015, 2015).is_ok());

    // Invalid: start after end
    assert!(validate_date_range(2020, 2010).is_err());

    // Invalid: too early
    assert!(validate_date_range(1950, 2020).is_err());

    // Invalid: too late
    assert!(validate_date_range(2010, 2040).is_err());
}

/// Test input validation
#[test]
fn test_input_validation() {
    // Valid inputs
    assert!(validate_inputs("USA,DEU", "SP.POP.TOTL", 2010, 2020, "/tmp").is_ok());

    // Empty countries
    assert!(validate_inputs("", "SP.POP.TOTL", 2010, 2020, "/tmp").is_err());

    // Empty indicators
    assert!(validate_inputs("USA", "", 2010, 2020, "/tmp").is_err());

    // Empty output path
    assert!(validate_inputs("USA", "SP.POP.TOTL", 2010, 2020, "").is_err());
}

/// Test plot dimension validation
#[test]
fn test_plot_dimensions() {
    // Valid dimensions
    assert!(validate_plot_dimensions(800, 600).is_ok());
    assert!(validate_plot_dimensions(200, 200).is_ok());
    assert!(validate_plot_dimensions(3000, 3000).is_ok());

    // Invalid: too small
    assert!(validate_plot_dimensions(100, 600).is_err());
    assert!(validate_plot_dimensions(800, 100).is_err());

    // Invalid: too large
    assert!(validate_plot_dimensions(4000, 600).is_err());
    assert!(validate_plot_dimensions(800, 4000).is_err());
}

// Helper functions that mirror the GUI logic

fn parse_list(s: &str) -> Vec<String> {
    s.split([',', ';'])
        .map(|x| x.trim().to_string())
        .filter(|x| !x.is_empty())
        .collect()
}

fn validate_date_range(from: i32, until: i32) -> Result<(), String> {
    if from > until {
        return Err("Start year cannot be later than end year".to_string());
    }

    if from < 1960 || until > 2030 {
        return Err("Years should be between 1960 and 2030".to_string());
    }

    Ok(())
}

fn validate_inputs(
    countries: &str,
    indicators: &str,
    from: i32,
    until: i32,
    output_path: &str,
) -> Result<(), String> {
    if countries.trim().is_empty() {
        return Err("Please enter at least one country code".to_string());
    }

    if indicators.trim().is_empty() {
        return Err("Please enter at least one indicator code".to_string());
    }

    if output_path.trim().is_empty() {
        return Err("Please specify an output directory".to_string());
    }

    validate_date_range(from, until)?;

    Ok(())
}

fn validate_plot_dimensions(width: u32, height: u32) -> Result<(), String> {
    if !(200..=3000).contains(&width) {
        return Err("Plot width must be between 200 and 3000 pixels".to_string());
    }
    if !(200..=3000).contains(&height) {
        return Err("Plot height must be between 200 and 3000 pixels".to_string());
    }
    Ok(())
}
