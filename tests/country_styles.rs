//! Tests for country-consistent styling functionality

#[cfg(test)]
mod tests {
    use tempfile::NamedTempFile;
    use wbi_rs::models::DataPoint;
    use wbi_rs::viz::{LegendMode, PlotKind};

    fn create_test_data() -> Vec<DataPoint> {
        vec![
            DataPoint {
                country_iso3: "USA".to_string(),
                country_name: "United States".to_string(),
                country_id: "US".to_string(),
                indicator_id: "GDP".to_string(),
                indicator_name: "GDP".to_string(),
                year: 2020,
                value: Some(100.0),
                unit: None,
                obs_status: None,
                decimal: None,
            },
            DataPoint {
                country_iso3: "USA".to_string(),
                country_name: "United States".to_string(),
                country_id: "US".to_string(),
                indicator_id: "Population".to_string(),
                indicator_name: "Population".to_string(),
                year: 2020,
                value: Some(200.0),
                unit: None,
                obs_status: None,
                decimal: None,
            },
            DataPoint {
                country_iso3: "DEU".to_string(),
                country_name: "Germany".to_string(),
                country_id: "DE".to_string(),
                indicator_id: "GDP".to_string(),
                indicator_name: "GDP".to_string(),
                year: 2020,
                value: Some(150.0),
                unit: None,
                obs_status: None,
                decimal: None,
            },
            DataPoint {
                country_iso3: "DEU".to_string(),
                country_name: "Germany".to_string(),
                country_id: "DE".to_string(),
                indicator_id: "Population".to_string(),
                indicator_name: "Population".to_string(),
                year: 2020,
                value: Some(250.0),
                unit: None,
                obs_status: None,
                decimal: None,
            },
        ]
    }

    #[test]
    fn test_country_styles_enabled() {
        let data = create_test_data();
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().with_extension("svg");

        // Test with country styles enabled - color mode
        let result = wbi_rs::viz::plot_chart(
            &data,
            &path,
            800,
            600,
            "en",
            LegendMode::Right,
            "Country Styles Test",
            PlotKind::LinePoints,
            0.3,
            Some(true), // enable country styles
        );

        assert!(result.is_ok(), "Country styles plot should succeed");
        assert!(path.exists(), "SVG file should be created");

        // Read the SVG content to verify basic structure
        let svg_content = std::fs::read_to_string(&path).unwrap();
        assert!(svg_content.contains("<svg"), "Should contain SVG content");
        assert!(
            svg_content.contains("Country Styles Test"),
            "Should contain title"
        );
    }

    #[test]
    fn test_country_styles_symbols_mode() {
        let data = create_test_data();
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().with_extension("svg");

        // Test with country styles enabled - symbols mode
        let result = wbi_rs::viz::plot_chart(
            &data,
            &path,
            800,
            600,
            "en",
            LegendMode::Right,
            "Country Styles Symbols Test",
            PlotKind::LinePoints,
            0.3,
            Some(true), // enable country styles - symbols mode
        );

        assert!(result.is_ok(), "Country styles symbols plot should succeed");
        assert!(path.exists(), "SVG file should be created");

        // Read the SVG content to verify basic structure
        let svg_content = std::fs::read_to_string(&path).unwrap();
        assert!(svg_content.contains("<svg"), "Should contain SVG content");
        assert!(
            svg_content.contains("Country Styles Symbols Test"),
            "Should contain title"
        );
        // The legend should contain visual style elements for symbols mode
        // Check for presence of path elements (lines) and shape elements (markers)
        assert!(
            svg_content.contains("<path") || svg_content.contains("<circle") || svg_content.contains("<polygon"),
            "Should contain visual elements indicating proper legend rendering with shapes"
        );
    }

    #[test]
    fn test_country_styles_disabled() {
        let data = create_test_data();
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().with_extension("svg");

        // Test with country styles disabled (None means feature disabled)
        let result = wbi_rs::viz::plot_chart(
            &data,
            &path,
            800,
            600,
            "en",
            LegendMode::Right,
            "Normal Styles Test",
            PlotKind::LinePoints,
            0.3,
            None, // no country styles
        );

        assert!(result.is_ok(), "Normal plot should succeed");
        assert!(path.exists(), "SVG file should be created");

        // Read the SVG content to verify basic structure
        let svg_content = std::fs::read_to_string(&path).unwrap();
        assert!(svg_content.contains("<svg"), "Should contain SVG content");
        assert!(
            svg_content.contains("Normal Styles Test"),
            "Should contain title"
        );
    }

    #[test]
    fn test_deterministic_country_styling() {
        let data = create_test_data();

        // Create two identical plots and verify they are the same
        let temp_file1 = NamedTempFile::new().unwrap();
        let path1 = temp_file1.path().with_extension("svg");

        let temp_file2 = NamedTempFile::new().unwrap();
        let path2 = temp_file2.path().with_extension("svg");

        // Plot 1
        wbi_rs::viz::plot_chart(
            &data,
            &path1,
            800,
            600,
            "en",
            LegendMode::Right,
            "Deterministic Test 1",
            PlotKind::LinePoints,
            0.3,
            Some(true),
        )
        .unwrap();

        // Plot 2 (same data, same settings)
        wbi_rs::viz::plot_chart(
            &data,
            &path2,
            800,
            600,
            "en",
            LegendMode::Right,
            "Deterministic Test 1", // same title to ensure identical output
            PlotKind::LinePoints,
            0.3,
            Some(true),
        )
        .unwrap();

        let svg1 = std::fs::read_to_string(&path1).unwrap();
        let svg2 = std::fs::read_to_string(&path2).unwrap();

        // The SVG content should be identical for identical inputs
        assert_eq!(
            svg1, svg2,
            "Identical inputs should produce identical SVG output"
        );
    }
}
