use anyhow::{Result, bail};
use clap::{Args, Parser, Subcommand, ValueEnum};
use num_format::{Locale, ToFormattedString};
use std::path::{Path, PathBuf};
use world_bank_data_rust::{Client, DateSpec};
use world_bank_data_rust::{stats, storage, viz};

#[derive(Parser, Debug)]
#[command(
    name = "world_bank_data_rust",
    version,
    about = "Fetch, store, visualize & summarize World Bank indicators"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Fetch data (and optionally save, plot, and print stats).
    Get(GetArgs),
}

#[derive(Clone, Copy, Debug, clap::ValueEnum, PartialEq, Eq)]
pub enum OutFormat {
    Csv,
    Json,
}

#[derive(ValueEnum, Clone, Debug)]
enum LegendPos {
    Inside,
    Right,
    Top,
    Bottom,
}

#[derive(ValueEnum, Clone, Debug)]
enum PlotKindArg {
    Line,
    Scatter,
    LinePoints,
    Area,
    StackedArea,
    GroupedBar,
    Loess,
}

#[derive(Args, Debug)]
struct GetArgs {
    /// Country/region codes separated by comma or semicolon (e.g., DEU,USA or EUU)
    #[arg(short, long)]
    countries: String,
    /// Indicator codes separated by comma or semicolon (e.g., SP.POP.TOTL)
    #[arg(short, long)]
    indicators: String,
    /// Year (YYYY) or range (YYYY:YYYY)
    #[arg(short = 'd', long)]
    date: Option<String>,
    /// Source id (e.g., 2 for WDI). Required by API when requesting multiple indicators.
    #[arg(long)]
    source: Option<u32>,
    /// Save results to file (format inferred by --format or extension).
    #[arg(long)]
    out: Option<PathBuf>,
    /// Output format (csv or json). If omitted, inferred from --out extension.
    #[arg(long, value_enum)]
    format: Option<OutFormat>,
    /// Create a chart at the given path (.svg or .png).
    #[arg(long)]
    plot: Option<PathBuf>,
    /// Width of the plot (default 1000).
    #[arg(long, default_value_t = 1000)]
    width: u32,
    /// Height of the plot (default 600).
    #[arg(long, default_value_t = 600)]
    height: u32,
    /// Title for the chart (defaults to "World Bank Indicator(s)")
    #[arg(long)]
    title: Option<String>,
    /// Print grouped statistics to stdout.
    #[arg(long, default_value_t = false)]
    stats: bool,
    /// Locale for number formatting in chart labels & stats (e.g., en, de, fr). Default: en
    #[arg(long, default_value = "en")]
    locale: String,
    /// Legend placement: inside (overlay), right (panel), top (band), or bottom (band).
    /// Default: right
    #[arg(long, value_enum, default_value = "right")]
    legend: LegendPos,
    /// Chart type: line, scatter, line-points, or area (default: line)
    #[arg(long = "plot-kind", value_enum, default_value = "line")]
    plot_kind: PlotKindArg,
    /// LOESS span in (0,1]; fraction of neighbors used (only for --plot-kind loess)
    #[arg(long = "loess-span", default_value_t = 0.3, value_parser = parse_loess_span)]
    loess_span: f64,
}

fn parse_list(s: &str) -> Vec<String> {
    s.split([',', ';'])
        .map(|x| x.trim().to_string())
        .filter(|x| !x.is_empty())
        .collect()
}

fn parse_date(s: &str) -> Option<DateSpec> {
    if let Some((a, b)) = s.split_once(':') {
        let start = a.parse::<i32>().ok()?;
        let end = b.parse::<i32>().ok()?;
        Some(DateSpec::Range { start, end })
    } else {
        s.parse::<i32>().ok().map(DateSpec::Year)
    }
}

fn map_locale(tag: &str) -> (&'static Locale, char) {
    match tag.to_lowercase().as_str() {
        "de" | "de_de" | "german" => (&Locale::de, ','),
        "fr" | "fr_fr" => (&Locale::fr, ','),
        "es" | "es_es" => (&Locale::es, ','),
        "it" | "it_it" => (&Locale::it, ','),
        "pt" | "pt_pt" | "pt_br" => (&Locale::pt, ','),
        "nl" | "nl_nl" => (&Locale::nl, ','),
        _ => (&Locale::en, '.'),
    }
}

fn fmt_float_with_locale(x: f64, loc: &Locale, dec_sep: char) -> String {
    let mut s = format!("{:.4}", x);
    if s.contains('.') {
        while s.ends_with('0') {
            s.pop();
        }
        if s.ends_with('.') {
            s.pop();
        }
    }
    if let Some((intp, fracp)) = s.split_once('.') {
        let sign = if intp.starts_with('-') { "-" } else { "" };
        let digits = intp.trim_start_matches('-');
        let int_num: i64 = digits.parse().unwrap_or(0);
        let grouped = int_num.to_formatted_string(loc);
        if fracp.is_empty() {
            format!("{}{}", sign, grouped)
        } else {
            format!("{}{}{}{}", sign, grouped, dec_sep, fracp)
        }
    } else {
        let sign = if s.starts_with('-') { "-" } else { "" };
        let digits = s.trim_start_matches('-');
        let int_num: i64 = digits.parse().unwrap_or(0);
        let grouped = int_num.to_formatted_string(loc);
        format!("{}{}", sign, grouped)
    }
}

fn fmt_opt_locale(v: Option<f64>, loc: &Locale, dec_sep: char) -> String {
    match v {
        Some(x) if x.is_finite() => fmt_float_with_locale(x, loc, dec_sep),
        _ => "NA".to_string(),
    }
}

// Keep using your existing enum OutFormat { Csv, Json }.
// No need for PartialEq; we use pattern matching.
fn decide_output_format(path: &Path, format_flag: Option<OutFormat>) -> Result<&'static str> {
    // If both a flag and an extension are present, ensure they don't conflict.
    if let (Some(fmt_flag), Some(ext)) = (format_flag, path.extension().and_then(|e| e.to_str())) {
        match (ext.to_ascii_lowercase().as_str(), fmt_flag) {
            ("csv", OutFormat::Json) | ("json", OutFormat::Csv) => {
                bail!(
                    "Format conflict: --format {:?} but output extension '.{}'. \
                     Align them or omit --format.",
                    fmt_flag,
                    ext
                );
            }
            _ => {}
        }
    }

    // Decide final format
    let fmt = match (format_flag, path.extension().and_then(|e| e.to_str())) {
        (Some(OutFormat::Csv), _) => "csv",
        (Some(OutFormat::Json), _) => "json",
        (None, Some(ext)) => match ext.to_ascii_lowercase().as_str() {
            "csv" => "csv",
            "json" => "json",
            other => bail!(
                "Unknown output extension '.{}'. Use .csv/.json or pass --format csv|json.",
                other
            ),
        },
        (None, None) => "csv", // default if no extension and no --format
    };

    Ok(fmt)
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Command::Get(args) => cmd_get(args),
    }
}

fn cmd_get(args: GetArgs) -> Result<()> {
    let client = Client::default();
    let countries = parse_list(&args.countries);
    let indicators = parse_list(&args.indicators);
    let date = match &args.date {
        Some(s) => parse_date(s)
            .ok_or_else(|| anyhow::anyhow!("invalid --date, expected YYYY or YYYY:YYYY"))?,
        None => DateSpec::Range {
            start: 2000,
            end: 2020,
        },
    };

    let points = client.fetch(&countries, &indicators, Some(date), args.source)?;

    if let Some(path) = args.out.as_ref() {
        let fmt = decide_output_format(path, args.format)?;
        match fmt {
            "csv" => storage::save_csv(&points, path)?,
            "json" => storage::save_json(&points, path)?,
            other => anyhow::bail!("unsupported format: {}", other),
        }
        eprintln!("Saved {} rows to {}", points.len(), path.display());
    }

    if let Some(plot_path) = args.plot.as_ref() {
        let legend_mode = match args.legend {
            LegendPos::Inside => viz::LegendMode::Inside,
            LegendPos::Right => viz::LegendMode::Right,
            LegendPos::Top => viz::LegendMode::Top,
            LegendPos::Bottom => viz::LegendMode::Bottom,
        };
        let title = args.title.as_deref().unwrap_or("World Bank Indicator(s)");
        let plot_kind = match args.plot_kind {
            PlotKindArg::Line => viz::PlotKind::Line,
            PlotKindArg::Scatter => viz::PlotKind::Scatter,
            PlotKindArg::LinePoints => viz::PlotKind::LinePoints,
            PlotKindArg::Area => viz::PlotKind::Area,
            PlotKindArg::StackedArea => viz::PlotKind::StackedArea,
            PlotKindArg::GroupedBar => viz::PlotKind::GroupedBar,
            PlotKindArg::Loess => viz::PlotKind::Loess,
        };
        viz::plot_chart(
            &points,
            plot_path,
            args.width,
            args.height,
            &args.locale,
            legend_mode,
            title,
            plot_kind,
            args.loess_span,
        )?;
        eprintln!("Wrote plot to {}", plot_path.display());
    }

    if args.stats {
        let (loc, dec_sep) = map_locale(&args.locale);
        let summaries = stats::grouped_summary(&points);
        for s in summaries {
            println!(
                "{} • {}  count={} missing={}  min={} max={} mean={} median={}",
                s.key.country_iso3,
                s.key.indicator_id,
                s.count,
                s.missing,
                fmt_opt_locale(s.min, loc, dec_sep),
                fmt_opt_locale(s.max, loc, dec_sep),
                fmt_opt_locale(s.mean, loc, dec_sep),
                fmt_opt_locale(s.median, loc, dec_sep),
            );
        }
    }

    Ok(())
}

/// Validate `--loess-span` ∈ (0, 1].
fn parse_loess_span(s: &str) -> Result<f64, String> {
    let x: f64 = s
        .parse()
        .map_err(|_| "invalid float for --loess-span".to_string())?;
    if x <= 0.0 || x > 1.0 {
        Err("loess span must be in (0, 1]".into())
    } else {
        Ok(x)
    }
}

#[cfg(test)]
mod tests_out_format {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn ext_csv_no_flag_yields_csv() {
        let p = PathBuf::from("pop.csv");
        let got = decide_output_format(&p, None).expect("inference should succeed");
        assert_eq!(got, "csv");
    }

    #[test]
    fn ext_json_no_flag_yields_json() {
        let p = PathBuf::from("pop.json");
        let got = decide_output_format(&p, None).expect("inference should succeed");
        assert_eq!(got, "json");
    }

    #[test]
    fn no_ext_no_flag_defaults_to_csv() {
        let p = PathBuf::from("pop");
        let got = decide_output_format(&p, None).expect("default should be csv");
        assert_eq!(got, "csv");
    }

    #[test]
    fn matching_flag_and_ext_is_ok_csv() {
        let p = PathBuf::from("data.csv");
        let got = decide_output_format(&p, Some(OutFormat::Csv)).expect("should match");
        assert_eq!(got, "csv");
    }

    #[test]
    fn matching_flag_and_ext_is_ok_json() {
        let p = PathBuf::from("data.json");
        let got = decide_output_format(&p, Some(OutFormat::Json)).expect("should match");
        assert_eq!(got, "json");
    }

    #[test]
    fn conflict_between_flag_and_ext_errors() {
        let p = PathBuf::from("data.csv");
        let err = decide_output_format(&p, Some(OutFormat::Json)).unwrap_err();
        let msg = format!("{err:#}");
        assert!(
            msg.to_lowercase().contains("conflict"),
            "unexpected error msg: {msg}"
        );
    }

    #[test]
    fn unknown_ext_with_explicit_flag_is_allowed_and_uses_flag() {
        let p = PathBuf::from("data.xyz");
        let got =
            decide_output_format(&p, Some(OutFormat::Csv)).expect("explicit format should win");
        assert_eq!(got, "csv");
    }

    #[test]
    fn unknown_ext_without_flag_errors() {
        let p = PathBuf::from("data.xyz");
        let err = decide_output_format(&p, None).unwrap_err();
        let msg = format!("{err:#}");
        assert!(
            msg.to_lowercase().contains("unknown output extension"),
            "unexpected error msg: {msg}"
        );
    }

    #[test]
    fn uppercase_extension_is_supported() {
        let p = PathBuf::from("DATA.CSV");
        let got = decide_output_format(&p, None).expect("inference should succeed");
        assert_eq!(got, "csv");
    }
}
