/*!
 * GUI application for wbi-rs - World Bank Indicators data fetcher and visualizer
 *
 * A cross-platform desktop application providing an intuitive interface for:
 * - Selecting countries and indicators
 * - Configuring date ranges and export options
 * - Generating charts and exporting data
 *
 * Platform support: Windows, macOS, Linux
 */

use anyhow::Result;
use eframe::egui;
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use wbi_rs::viz::{LegendMode, PlotKind};
use wbi_rs::{Client, DateSpec, storage, viz};

fn main() -> Result<(), eframe::Error> {
    // Enable logging for better debugging
    env_logger::init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_min_inner_size([600.0, 400.0])
            .with_title("World Bank Indicators - wbi-rs"),
        ..Default::default()
    };

    eframe::run_native(
        "World Bank Indicators",
        options,
        Box::new(|_cc| Ok(Box::new(WbiApp::new()))),
    )
}

/// Main application state
struct WbiApp {
    // Input fields
    countries: String,
    indicators: String,
    date_from: i32,
    date_until: i32,

    // Export options
    export_format: ExportFormat,
    output_path: String,
    create_plot: bool,
    plot_format: PlotFormat,
    plot_width: u32,
    plot_height: u32,

    // Advanced options
    source_id: String,
    plot_title: String,
    locale: String,
    legend_position: LegendPosition,
    plot_kind: PlotKindOption,
    country_styles: bool,

    // UI state
    is_loading: bool,
    status_message: String,
    error_message: String,

    // Background operation
    operation_receiver: Option<mpsc::Receiver<OperationResult>>,
}

#[derive(Debug, Clone, PartialEq)]
enum ExportFormat {
    Csv,
    Json,
    Both,
}

#[derive(Debug, Clone, PartialEq)]
enum PlotFormat {
    Png,
    Svg,
}

#[derive(Debug, Clone, PartialEq)]
enum LegendPosition {
    Bottom,
    Right,
    Top,
    Inside,
}

#[derive(Debug, Clone, PartialEq)]
enum PlotKindOption {
    Line,
    Scatter,
    LinePoints,
    Area,
    StackedArea,
    GroupedBar,
    Loess,
}

#[derive(Debug)]
enum OperationResult {
    Success(String),
    Error(String),
}

impl WbiApp {
    fn new() -> Self {
        // Default to user's home directory for output
        let home_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .to_string_lossy()
            .to_string();

        Self {
            countries: String::new(),
            indicators: String::new(),
            date_from: 2010,
            date_until: 2020,

            export_format: ExportFormat::Csv,
            output_path: home_dir,
            create_plot: false,
            plot_format: PlotFormat::Png,
            plot_width: 1000,
            plot_height: 600,

            source_id: String::new(),
            plot_title: String::new(),
            locale: "en".to_string(),
            legend_position: LegendPosition::Bottom,
            plot_kind: PlotKindOption::Line,
            country_styles: false,

            is_loading: false,
            status_message: String::new(),
            error_message: String::new(),
            operation_receiver: None,
        }
    }

    fn validate_inputs(&self) -> Result<()> {
        if self.countries.trim().is_empty() {
            anyhow::bail!("Please enter at least one country code (e.g., USA, DEU, CHN)");
        }

        if self.indicators.trim().is_empty() {
            anyhow::bail!("Please enter at least one indicator code (e.g., SP.POP.TOTL)");
        }

        // Validate date range
        if self.date_from > self.date_until {
            anyhow::bail!("Start year cannot be later than end year");
        }

        if self.date_from < 1960 || self.date_until > 2030 {
            anyhow::bail!("Years should be between 1960 and 2030");
        }

        // Validate output path
        if self.output_path.trim().is_empty() {
            anyhow::bail!("Please specify an output directory");
        }

        // Validate plot dimensions if creating plot
        if self.create_plot {
            if self.plot_width < 200 || self.plot_width > 3000 {
                anyhow::bail!("Plot width must be between 200 and 3000 pixels");
            }
            if self.plot_height < 200 || self.plot_height > 3000 {
                anyhow::bail!("Plot height must be between 200 and 3000 pixels");
            }
        }

        Ok(())
    }

    fn start_operation(&mut self) {
        if let Err(err) = self.validate_inputs() {
            self.error_message = format!("Validation error: {}", err);
            return;
        }

        self.is_loading = true;
        self.error_message.clear();
        self.status_message = "Fetching data from World Bank API...".to_string();

        let (sender, receiver) = mpsc::channel();
        self.operation_receiver = Some(receiver);

        // Clone the data we need for the background thread
        let countries = parse_list(&self.countries);
        let indicators = parse_list(&self.indicators);
        let date_from = self.date_from;
        let date_until = self.date_until;
        let date_spec = if date_from == date_until {
            DateSpec::Year(date_from)
        } else {
            DateSpec::Range {
                start: date_from,
                end: date_until,
            }
        };

        let source_id = if self.source_id.trim().is_empty() {
            None
        } else {
            self.source_id.parse().ok()
        };

        let export_format = self.export_format.clone();
        let output_path = self.output_path.clone();
        let create_plot = self.create_plot;
        let plot_format = self.plot_format.clone();
        let plot_width = self.plot_width;
        let plot_height = self.plot_height;
        let plot_title = if self.plot_title.trim().is_empty() {
            "World Bank Indicators".to_string()
        } else {
            self.plot_title.clone()
        };
        let locale = self.locale.clone();
        let legend_position = self.legend_position.clone();
        let plot_kind = self.plot_kind.clone();
        let country_styles = self.country_styles;

        // Spawn background thread for the operation
        thread::spawn(move || {
            let plot_config = if create_plot {
                Some(PlotConfig {
                    format: plot_format,
                    width: plot_width,
                    height: plot_height,
                    title: plot_title,
                    locale,
                    legend_position,
                    kind: plot_kind,
                    country_styles,
                })
            } else {
                None
            };

            let config = OperationConfig {
                export_format,
                output_path,
                plot_config,
            };

            let result = perform_operation(countries, indicators, date_spec, source_id, config);

            let _ = sender.send(result);
        });
    }

    fn check_operation_result(&mut self) {
        if let Some(receiver) = &self.operation_receiver
            && let Ok(result) = receiver.try_recv()
        {
            self.is_loading = false;
            self.operation_receiver = None;

            match result {
                OperationResult::Success(message) => {
                    self.status_message = message;
                    self.error_message.clear();
                }
                OperationResult::Error(error) => {
                    self.error_message = error;
                    self.status_message.clear();
                }
            }
        }
    }
}

impl eframe::App for WbiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Check for completed background operations
        self.check_operation_result();

        // Request repaint if loading (for spinner animation)
        if self.is_loading {
            ctx.request_repaint();
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("World Bank Indicators Data Tool");
                ui.add_space(10.0);

                // Main input section
                ui.group(|ui| {
                    ui.label("Data Selection");
                    ui.add_space(5.0);

                    ui.horizontal(|ui| {
                        ui.label("Countries:");
                        ui.text_edit_singleline(&mut self.countries)
                            .on_hover_text("Enter country codes separated by commas (e.g., USA,DEU,CHN)");
                    });

                    ui.horizontal(|ui| {
                        ui.label("Indicators:");
                        ui.text_edit_singleline(&mut self.indicators)
                            .on_hover_text("Enter indicator codes separated by commas (e.g., SP.POP.TOTL,NY.GDP.MKTP.CD)");
                    });

                    ui.horizontal(|ui| {
                        ui.label("Date range:");
                        ui.add(egui::DragValue::new(&mut self.date_from).range(1960..=2030));
                        ui.label("to");
                        ui.add(egui::DragValue::new(&mut self.date_until).range(1960..=2030));
                    });
                });

                ui.add_space(10.0);

                // Export options section
                ui.group(|ui| {
                    ui.label("Export Options");
                    ui.add_space(5.0);

                    ui.horizontal(|ui| {
                        ui.label("Format:");
                        ui.radio_value(&mut self.export_format, ExportFormat::Csv, "CSV");
                        ui.radio_value(&mut self.export_format, ExportFormat::Json, "JSON");
                        ui.radio_value(&mut self.export_format, ExportFormat::Both, "Both");
                    });

                    ui.horizontal(|ui| {
                        ui.label("Output path:");
                        ui.text_edit_singleline(&mut self.output_path);
                        if ui.button("Browse").clicked()
                            && let Some(path) = rfd::FileDialog::new().pick_folder() {
                            self.output_path = path.to_string_lossy().to_string();
                        }
                    });

                    ui.checkbox(&mut self.create_plot, "Create chart");

                    if self.create_plot {
                        ui.horizontal(|ui| {
                            ui.label("Chart format:");
                            ui.radio_value(&mut self.plot_format, PlotFormat::Png, "PNG");
                            ui.radio_value(&mut self.plot_format, PlotFormat::Svg, "SVG");
                        });

                        ui.horizontal(|ui| {
                            ui.label("Dimensions:");
                            ui.add(egui::DragValue::new(&mut self.plot_width).range(200..=3000));
                            ui.label("×");
                            ui.add(egui::DragValue::new(&mut self.plot_height).range(200..=3000));
                            ui.label("pixels");
                        });
                    }
                });

                ui.add_space(10.0);

                // Advanced options (collapsible)
                ui.collapsing("Advanced Options", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Source ID:");
                        ui.text_edit_singleline(&mut self.source_id)
                            .on_hover_text("Optional source ID (e.g., 2 for WDI). Required for multiple indicators.");
                    });

                    if self.create_plot {
                        ui.horizontal(|ui| {
                            ui.label("Chart title:");
                            ui.text_edit_singleline(&mut self.plot_title)
                                .on_hover_text("Custom title for the chart");
                        });

                        ui.horizontal(|ui| {
                            ui.label("Locale:");
                            egui::ComboBox::from_label("")
                                .selected_text(&self.locale)
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut self.locale, "en".to_string(), "English (en)");
                                    ui.selectable_value(&mut self.locale, "de".to_string(), "German (de)");
                                    ui.selectable_value(&mut self.locale, "fr".to_string(), "French (fr)");
                                    ui.selectable_value(&mut self.locale, "es".to_string(), "Spanish (es)");
                                    ui.selectable_value(&mut self.locale, "it".to_string(), "Italian (it)");
                                });
                        });

                        ui.horizontal(|ui| {
                            ui.label("Legend position:");
                            egui::ComboBox::from_label("")
                                .selected_text(format!("{:?}", self.legend_position))
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut self.legend_position, LegendPosition::Bottom, "Bottom");
                                    ui.selectable_value(&mut self.legend_position, LegendPosition::Right, "Right");
                                    ui.selectable_value(&mut self.legend_position, LegendPosition::Top, "Top");
                                    ui.selectable_value(&mut self.legend_position, LegendPosition::Inside, "Inside");
                                });
                        });

                        ui.horizontal(|ui| {
                            ui.label("Chart type:");
                            egui::ComboBox::from_label("")
                                .selected_text(format!("{:?}", self.plot_kind))
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut self.plot_kind, PlotKindOption::Line, "Line");
                                    ui.selectable_value(&mut self.plot_kind, PlotKindOption::Scatter, "Scatter");
                                    ui.selectable_value(&mut self.plot_kind, PlotKindOption::LinePoints, "Line + Points");
                                    ui.selectable_value(&mut self.plot_kind, PlotKindOption::Area, "Area");
                                    ui.selectable_value(&mut self.plot_kind, PlotKindOption::StackedArea, "Stacked Area");
                                    ui.selectable_value(&mut self.plot_kind, PlotKindOption::GroupedBar, "Grouped Bar");
                                    ui.selectable_value(&mut self.plot_kind, PlotKindOption::Loess, "Loess");
                                });
                        });

                        ui.checkbox(&mut self.country_styles, "Use country-consistent styling")
                            .on_hover_text("Same countries use consistent colors across different indicators");
                    }
                });

                ui.add_space(15.0);

                // Action buttons
                ui.horizontal(|ui| {
                    if ui.add_enabled(!self.is_loading, egui::Button::new("Fetch Data")).clicked() {
                        self.start_operation();
                    }

                    if self.is_loading {
                        ui.spinner();
                        ui.label("Processing...");
                    }
                });

                ui.add_space(10.0);

                // Status messages
                if !self.status_message.is_empty() {
                    ui.colored_label(egui::Color32::DARK_GREEN, &self.status_message);
                }

                if !self.error_message.is_empty() {
                    ui.colored_label(egui::Color32::RED, &self.error_message);
                }
            });
        });
    }
}

fn parse_list(s: &str) -> Vec<String> {
    s.split([',', ';'])
        .map(|x| x.trim().to_string())
        .filter(|x| !x.is_empty())
        .collect()
}

#[derive(Debug)]
struct OperationConfig {
    export_format: ExportFormat,
    output_path: String,
    plot_config: Option<PlotConfig>,
}

#[derive(Debug)]
struct PlotConfig {
    format: PlotFormat,
    width: u32,
    height: u32,
    title: String,
    locale: String,
    legend_position: LegendPosition,
    kind: PlotKindOption,
    country_styles: bool,
}

fn perform_operation(
    countries: Vec<String>,
    indicators: Vec<String>,
    date_spec: DateSpec,
    source_id: Option<u32>,
    config: OperationConfig,
) -> OperationResult {
    // Fetch data
    let client = Client::default();
    let points = match client.fetch(&countries, &indicators, Some(date_spec), source_id) {
        Ok(points) => points,
        Err(err) => return OperationResult::Error(format!("Failed to fetch data: {}", err)),
    };

    if points.is_empty() {
        return OperationResult::Error(
            "No data returned from the API. Please check your country and indicator codes."
                .to_string(),
        );
    }

    let mut output_files = Vec::new();

    // Export data
    let output_dir = PathBuf::from(&config.output_path);

    match config.export_format {
        ExportFormat::Csv | ExportFormat::Both => {
            let csv_path = output_dir.join("wbi_data.csv");
            if let Err(err) = storage::save_csv(&points, &csv_path) {
                return OperationResult::Error(format!("Failed to save CSV: {}", err));
            }
            output_files.push(csv_path.to_string_lossy().to_string());
        }
        _ => {}
    }

    match config.export_format {
        ExportFormat::Json | ExportFormat::Both => {
            let json_path = output_dir.join("wbi_data.json");
            if let Err(err) = storage::save_json(&points, &json_path) {
                return OperationResult::Error(format!("Failed to save JSON: {}", err));
            }
            output_files.push(json_path.to_string_lossy().to_string());
        }
        _ => {}
    }

    // Create plot if requested
    if let Some(plot_config) = config.plot_config {
        let plot_extension = match plot_config.format {
            PlotFormat::Png => "png",
            PlotFormat::Svg => "svg",
        };
        let plot_path = output_dir.join(format!("wbi_chart.{}", plot_extension));

        // Convert GUI enums to library types
        let legend_mode = match plot_config.legend_position {
            LegendPosition::Bottom => LegendMode::Bottom,
            LegendPosition::Right => LegendMode::Right,
            LegendPosition::Top => LegendMode::Top,
            LegendPosition::Inside => LegendMode::Inside,
        };

        let plot_kind_lib = match plot_config.kind {
            PlotKindOption::Line => PlotKind::Line,
            PlotKindOption::Scatter => PlotKind::Scatter,
            PlotKindOption::LinePoints => PlotKind::LinePoints,
            PlotKindOption::Area => PlotKind::Area,
            PlotKindOption::StackedArea => PlotKind::StackedArea,
            PlotKindOption::GroupedBar => PlotKind::GroupedBar,
            PlotKindOption::Loess => PlotKind::Loess,
        };

        let country_styles_option = if plot_config.country_styles {
            Some(true)
        } else {
            None
        };

        if let Err(err) = viz::plot_chart(
            &points,
            plot_path.to_str().unwrap(),
            plot_config.width,
            plot_config.height,
            &plot_config.locale,
            legend_mode,
            &plot_config.title,
            plot_kind_lib,
            0.3, // loess_span
            country_styles_option,
        ) {
            return OperationResult::Error(format!("Failed to create chart: {}", err));
        }

        output_files.push(plot_path.to_string_lossy().to_string());
    }

    let mut message = format!("Successfully processed {} data points!", points.len());
    if !output_files.is_empty() {
        message.push_str(&format!("\n\nFiles created:\n{}", output_files.join("\n")));
    }

    OperationResult::Success(message)
}
