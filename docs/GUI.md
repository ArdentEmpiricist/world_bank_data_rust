# wbi-rs GUI Application

The `wbi-gui` application provides a user-friendly graphical interface for fetching, visualizing, and exporting World Bank Indicators data. This modern, cross-platform desktop application makes it easy to work with World Bank data without requiring command-line knowledge.

## Features

### Core Functionality
- **Data Fetching**: Connect to the World Bank API to retrieve indicator data
- **Country Selection**: Input multiple country codes (e.g., USA, DEU, CHN)
- **Indicator Selection**: Specify one or more World Bank indicators (e.g., SP.POP.TOTL)
- **Date Range**: Select start and end years for your data
- **Export Options**: Save data as CSV, JSON, or both formats
- **Chart Generation**: Create professional charts in PNG or SVG format

### Advanced Options
- **Source ID**: Specify data source for multi-indicator requests
- **Chart Customization**: 
  - Custom titles and dimensions
  - Multiple chart types (line, scatter, area, etc.)
  - Country-consistent styling
  - Configurable legends and locales
- **Output Management**: Choose output directory with home folder default

## Installation and Usage

### Building from Source

1. Ensure you have Rust installed (https://rustup.rs/)
2. Clone the repository:
   ```bash
   git clone https://github.com/ArdentEmpiricist/wbi-rs
   cd wbi-rs
   ```
3. Build the GUI application:
   ```bash
   cargo build --release --bin wbi-gui
   ```
4. Run the application:
   ```bash
   cargo run --release --bin wbi-gui
   ```

### Using the Application

#### Basic Usage
1. **Enter Countries**: Type country codes separated by commas (e.g., "USA,DEU,CHN")
2. **Enter Indicators**: Type indicator codes separated by commas (e.g., "SP.POP.TOTL,NY.GDP.MKTP.CD")
3. **Set Date Range**: Use the spinners to select start and end years
4. **Choose Export Format**: Select CSV, JSON, or both
5. **Select Output Path**: Choose where to save your files (defaults to home directory)
6. **Fetch Data**: Click the "Fetch Data" button to download and process data

#### Creating Charts
1. Check the "Create chart" option
2. Choose format (PNG or SVG)
3. Set chart dimensions in pixels
4. Configure advanced chart options if needed

#### Advanced Options
Click "Advanced Options" to access:
- **Source ID**: Required for multi-indicator requests (e.g., "2" for WDI)
- **Chart Title**: Custom title for your chart
- **Locale**: Number formatting (English, German, French, etc.)
- **Legend Position**: Bottom, right, top, or inside the chart
- **Chart Type**: Line, scatter, area, bar charts, and more
- **Country Styling**: Consistent colors for the same country across indicators

## Platform Support

The GUI application is built with cross-platform compatibility:
- **Windows**: Fully supported with native look and feel
- **macOS**: Native Cocoa interface
- **Linux**: Works with X11 and Wayland display servers

## Error Handling

The application provides clear feedback for:
- Invalid input formats
- Network connectivity issues
- API errors
- File system problems

Error messages are displayed in red at the bottom of the interface, while success messages appear in green.

## Technical Details

### Architecture
- Built with **egui** for immediate-mode GUI
- Uses **eframe** for native application framework
- Integrates with existing wbi-rs library functions
- Background processing prevents UI freezing during API calls

### Performance
- Asynchronous data fetching
- Progress indication during long operations
- Efficient memory usage for large datasets
- Responsive UI even with heavy data processing

### Dependencies
- **egui/eframe**: Modern Rust GUI framework
- **rfd**: Native file dialogs
- **dirs**: Platform-specific directory access
- **env_logger**: Debugging and logging support

## Troubleshooting

### Common Issues

**GUI doesn't start**
- Ensure you have appropriate graphics drivers
- On Linux, you may need to install graphics libraries
- Check console output for specific error messages

**Network errors**
- Verify internet connectivity
- Check if corporate firewalls block World Bank API access
- Ensure system time is correct for API authentication

**File export issues**
- Verify write permissions to output directory
- Ensure sufficient disk space
- Check that file paths are valid for your operating system

### Getting Help

For technical issues:
1. Check the console output for detailed error messages
2. Review the [main documentation](../README.md)
3. Open an issue on the GitHub repository

## Examples

### Basic Population Data
1. Countries: `USA,CHN,IND`
2. Indicators: `SP.POP.TOTL`
3. Date Range: 2010 to 2020
4. Export: CSV
5. Create Chart: Yes (PNG format)

### Economic Indicators with Styling
1. Countries: `DEU,FRA,ITA,ESP`
2. Indicators: `NY.GDP.MKTP.CD,NY.GDP.PCAP.CD`
3. Advanced Options:
   - Source ID: `2`
   - Chart Type: Line + Points
   - Country Styling: Enabled
   - Legend: Right

This will create a professional chart with consistent colors for each country while differentiating between GDP total and per capita indicators.

## Development

### Testing
Run the GUI-specific tests:
```bash
cargo test --test gui
```

### Building
Build in release mode for optimal performance:
```bash
cargo build --release --bin wbi-gui
```

### Contributing
See the main [README.md](../README.md) for contribution guidelines.