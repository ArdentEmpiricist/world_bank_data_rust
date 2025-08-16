use anyhow::Result;
use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;
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

#[derive(ValueEnum, Clone, Debug)]
enum OutFormat {
    Csv,
    Json,
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
    /// Print grouped statistics to stdout.
    #[arg(long, default_value_t = false)]
    stats: bool,
}

fn fmt_opt(v: Option<f64>) -> String {
    match v {
        Some(x) if x.is_finite() => {
            // Format up to 4 decimals, then trim trailing zeros and trailing dot.
            let s = format!("{:.4}", x);
            s.trim_end_matches('0').trim_end_matches('.').to_string()
        }
        _ => "NA".to_string(),
    }
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
        let fmt = match args.format {
            Some(OutFormat::Csv) => "csv",
            Some(OutFormat::Json) => "json",
            None => path.extension().and_then(|e| e.to_str()).unwrap_or("csv"),
        }
        .to_ascii_lowercase();
        match fmt.as_str() {
            "csv" => storage::save_csv(&points, path)?,
            "json" => storage::save_json(&points, path)?,
            other => anyhow::bail!("unsupported format: {}", other),
        }
        eprintln!("Saved {} rows to {}", points.len(), path.display());
    }

    if let Some(plot_path) = args.plot.as_ref() {
        viz::plot_lines(&points, plot_path, args.width, args.height)?;
        eprintln!("Wrote plot to {}", plot_path.display());
    }

    if args.stats {
        let summaries = stats::grouped_summary(&points);
        for s in summaries {
            println!(
                "{} • {}  count={} missing={}  min={} max={} mean={} median={}",
                s.key.country_iso3,
                s.key.indicator_id,
                s.count,
                s.missing,
                fmt_opt(s.min),
                fmt_opt(s.max),
                fmt_opt(s.mean),
                fmt_opt(s.median)
            );
        }
    }

    Ok(())
}
