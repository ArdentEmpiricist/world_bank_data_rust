
[![Crates.io](https://img.shields.io/crates/v/wbi-rs?label=Crates.io)](https://crates.io/crates/wbi-rs)
[![rust-clippy analyze](https://img.shields.io/github/actions/workflow/status/ardentempiricist/wbi-rs/rust-clippy.yml?label=Rust%20Clippy)](https://github.com/ArdentEmpiricist/wbi-rs/actions/workflows/rust-clippy.yml)
[![Deploy](https://github.com/ArdentEmpiricist/wbi-rs/actions/workflows/deploy.yml/badge.svg)](https://github.com/ArdentEmpiricist/wbi-rs/actions/workflows/deploy.yml)
[![Documentation](https://docs.rs/wbi-rs/badge.svg)](https://docs.rs/wbi-rs/)
[![Crates.io](https://img.shields.io/crates/d/wbi-rs?color=darkblue&label=Downloads)](https://crates.io/crates/wbi-rs)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

# wbi-rs 🦀📊📈

<p align="center">
  <img src="https://github.com/ArdentEmpiricist/wbi-rs/blob/main/assets/logo.png?raw=true" alt="wbi-rs Logo" width="200"/>
</p>

Fetch, analyze, and visualize World Bank data from Rust.  
This project provides both a **CLI** and a **library API** to retrieve time series from the World Bank API, export them safely (CSV/JSON), compute grouped statistics, and render charts (SVG/PNG) with Plotters.

> Status: actively developed. Library API is stable enough for use; updates follow semantic versioning.

---

## Table of contents

- [Features](#features)
- [Install](#install)
- [Quick start (CLI)](#quick-start-cli)
- [CLI usage](#cli-usage)
  - [Format inference for `--out`](#format-inference-for---out)
  - [Examples](#examples)
- [Library usage](#library-usage)
  - [Add to `Cargo.toml`](#add-to-cargotoml)
  - [Fetch data](#fetch-data)
  - [Export data (atomic CSV/JSON)](#export-data-atomic-csvjson)
  - [Compute grouped summaries](#compute-grouped-summaries)
  - [Plot charts](#plot-charts)
  - [Data model](#data-model)
- [Data formats](#data-formats)
- [Security & reliability](#security--reliability)
- [Testing](#testing)
- [CI setup (example)](#ci-setup-example)
- [Contributing](#contributing)
- [Changelog (recent)](#changelog-recent)
- [License](#license)
- [Disclaimer](#disclaimer)

---

## Features

### What it can do

- **Retrieve data** from the World Bank by country/countries, indicator(s), and optional date range.
- **Multi-indicator requests** work without specifying a World Bank `source`; the client transparently fans out per indicator when `--source` is omitted, while still supporting the single-call path when `--source` is provided.
- **Show short stats in the terminal** (grouped min / max / mean / median per (indicator, country)).
- **Export datasets** to **CSV** or **JSON** (format inferred from `--out` extension or set via `--format`).  
  Exports are **atomic** and CSV is **spreadsheet-safe**.
- **Export plots** as **SVG** or **PNG** (backend inferred from `--plot` file extension).
- **Country-consistent styling**: when enabled via `--country-styles`, series from the same country share consistent base colors while indicators are differentiated by shades and patterns.

### Under the hood

- **Hardened HTTPS client** (rustls, connect/request timeouts, limited redirects, descriptive `User-Agent`: `wbi_rs`).
- **Robust URL handling**
- **Transient error resilience** (small retry/backoff) and **page caps**.
- **Valid JSON** under all inputs (`NaN`/`±∞` → `null`).
- **Numerical stability** (non-finite values treated as missing; safe float sorting).
- **Portable plotting** with an embedded TTF font for CI/headless environments.

---

## Install

Download a prebuilt binary (GitHub Releases):

1) Go to **GitHub → Releases**: [https://github.com/ArdentEmpiricist/wbi-rs/releases/new](https://github.com/ArdentEmpiricist/wbi-rs/releases/new)
2) Download the asset for your platform

Via cargo/crates.io:

```bash
cargo install wbi-rs
wbi --help
```

From source:

```bash
git clone <this-repo>
cd wbi-rs
cargo build --release
```

As a library (example):

```toml
[dependencies]
wbi-rs = "0.1" # or your path
anyhow = "1"
```

---

## Quick start (CLI)

Fetch population for Germany & France (2000–2023), write to CSV (format inferred from extension) and show quick stats in terminal:

```bash
wbi get \
  --countries DEU,FRA \
  --indicators SP.POP.TOTL \
  --date 2000:2023 \
  --out pop.csv \
  --stats
```

Output:
<p align="center">
  <img src="https://github.com/ArdentEmpiricist/wbi-rs/blob/main/assets/example_stats.png?raw=true" alt="example terminal output" style='width: 90%; object-fit: contain'/>
</p>

Fetch GDP (in current US$) for the US, China, Germany and India and render a plot (backend inferred from extension):

```bash
wbi get \
  --countries USA,CHN,DEU,IND \
  --indicators NY.GDP.MKTP.CD \
  --date 1970:2025 \
  --plot pop.svg \
  --plot-kind line-points
```

Output:
<p align="center">
  <img src="https://raw.githubusercontent.com/ArdentEmpiricist/wbi-rs/bf42ecfa7f9a4a559b886aa5bf48d397fb41233d/assets/example_plot.svg" alt="example plot" style='width: 90%; object-fit: contain'/>
</p>

### Country-Consistent Styling

You can enable country-consistent styling where all series for the same country share one base hue from the MS Office palette, while indicators within that country are differentiated by shades:

```bash
# Use country-consistent styling
wbi get \
  --countries USA,CHN,IND,DEU \
  --indicators SP.POP.TOTL,SP.POP.TOTL.FE.IN \
  --date 2010:2020 \
  --plot multi_indicator.svg \
  --plot-kind line-points \
  --country-styles
```

This feature ensures that:

- All series from the same country use the same base color hue
- Different indicators within a country are differentiated by brightness variations
- The styling is deterministic and consistent across runs

Output:
<p align="center">
  <img src="https://raw.githubusercontent.com/ArdentEmpiricist/wbi-rs/refs/heads/main/assets/example_multi_indicator.svg" alt="example plot" style='width: 90%; object-fit: contain'/>
</p>

---

## CLI usage

Subcommand `get` accepts at least:

- `--countries` ISO2/ISO3 codes separated by `,` or `;` (e.g., `DEU,FRA` or (attention: `" "` are required using `;`) `"DEU;FRA"`)
- `--indicators` World Bank indicator IDs (e.g., `SP.POP.TOTL`)
- `--date` optional year or range (e.g., `2020` or `2000:2023`)
- `--source` optional source ID (e.g., `2` for WDI). Recommended for efficiency when querying multiple indicators, but optional.
- `--out <PATH>` optional export (CSV/JSON); **atomic**
- `--plot <PATH>` optional chart output (SVG/PNG), using Plotters
- `--country-styles` enable country-consistent styling for multi-indicator plots at runtime

### Format inference for `--out`

- If `--format` is **not** provided, the format is **inferred**:
  - `.csv` → CSV
  - `.json` → JSON
  - no extension → defaults to CSV
  - unknown extension (with no `--format`) → error
- If `--format` **is** provided:
  - It must **match** known extensions; conflicting combinations (e.g., `--out data.csv --format json`) **error** early
  - For unknown extensions, the explicit format wins

### Examples

```bash
# CSV via extension inference
wbi get --countries DEU --indicators SP.POP.TOTL --out data.csv

# JSON via extension inference
wbi get --countries DEU --indicators SP.POP.TOTL --out data.json

# Unknown extension allowed when --format is explicit
wbi get --countries DEU --indicators SP.POP.TOTL --out dump.xyz --format csv

# Error on conflict
wbi get --countries DEU --indicators SP.POP.TOTL --out data.csv --format json
```

---

## Library usage

The crate exposes modules for API access, models, storage, statistics, and plotting.

### Add to `Cargo.toml`

```toml
[dependencies]
wbi-rs = "0.1" # or "{ path = "." } to your source"
anyhow = "1"
```

### Fetch data

```rust
use anyhow::Result;
use wbi_rs::api::Client;
use wbi_rs::models::DateSpec;

fn main() -> Result<()> {
    // Hardened blocking client (rustls, timeouts, redirect policy, UA)
    let api = Client::default();

    // Countries & indicators can be given as lists; date is optional
    let points = api.fetch(
        &["DEU".into(), "FRA".into()],
        &["SP.POP.TOTL".into()],
        Some(DateSpec::Range { start: 2000, end: 2023 }),
        None, // source id
    )?;

    println!("rows: {}", points.len());
    Ok(())
}
```

For multi-indicator requests when `source` is `None`, the client automatically handles fallback behavior by making separate requests per indicator to ensure data retrieval succeeds.

The `fetch` method automatically enriches `DataPoint.unit` values when observation rows lack a unit by fetching metadata from the World Bank indicator endpoint. This ensures that visualization and analysis code has access to appropriate unit information for axis labeling and scaling decisions.

If you need to manually fetch indicator units for specific indicators:

```rust
let units = api.fetch_indicator_units(&["SP.POP.TOTL".into(), "NY.GDP.MKTP.CD".into()])?;
// Returns HashMap<String, String> mapping indicator ID to unit
```

### Export data (atomic CSV/JSON)

```rust
use wbi_rs::storage::{save_csv, save_json};

save_csv(&points, "pop.csv")?;   // spreadsheet-safe + atomic
save_json(&points, "pop.json")?; // non-finite -> null + atomic
```

- **CSV**: fixed header order; cells beginning with `=`, `+`, `-`, `@` are prefixed with `'`.
- **JSON**: pretty-printed; non-finite floats are serialized as `null`.

Both writers use a tempfile in the destination directory and atomically replace the target file.

### Compute grouped summaries

```rust
use wbi_rs::stats::{grouped_summary, Summary};

let summaries: Vec<Summary> = grouped_summary(&points);
// Summary contains: key (indicator_id, country_iso3), count, missing, min, max, mean, median.
// Non-finite values are counted as missing; sorting avoids panics on floats.
```

### Plot charts

```rust
use wbi_rs::viz::plot_chart;

// `plot_chart` filters non-finite values and sorts by integer year.
// The backend is selected from the output extension (.svg, .png).
plot_chart(&points, "pop.svg")?;
```

Charts automatically derive appropriate units for axis labeling using a two-tier approach:

1. **Prefer units from DataPoint.unit**: When all points have the same non-empty unit, use it directly for axis labeling
2. **Fallback to indicator name parsing**: For single-indicator plots without consistent units, extract unit information from parentheses in the indicator name (e.g., "GDP (current US$)" → "current US$")

This approach ensures that both API-provided units and legacy indicator naming conventions are properly handled for visualization.

### Data model

```rust
// Simplified view
pub struct DataPoint {
    pub indicator_id: String,
    pub indicator_name: String,
    pub country_id: String,
    pub country_name: String,
    pub country_iso3: String,
    pub year: i32,
    pub value: Option<f64>,   // may be None for missing
    pub unit: Option<String>,
    pub obs_status: Option<String>,
    pub decimal: Option<i64>,
}
```

---

## Data formats

### CSV

- **Header:** `indicator_id, indicator_name, country_id, country_name, country_iso3, year, value, unit, obs_status, decimal`
- **Quoting/escaping:** handled by the `csv` crate (RFC-4180)
- **Missing values:** `None` → empty cell
- **Safety:** cells beginning with `=`, `+`, `-`, `@` are prefixed with `'` (prevents formula execution)

### JSON

- **Shape:** array of objects mirroring the CSV fields
- **Numbers:** non-finite floats serialized as `null`
- **Formatting:** pretty-printed for readability

---

## Security & reliability

- **Networking**
  - TLS via `rustls`
  - Request + connect timeouts, limited redirects
  - Descriptive `User-Agent`
  - Percent-encoding for user-supplied path segments
  - Small retry/backoff on transient failures
  - Hard cap on pages to avoid runaway jobs
- **Exports**
  - **Atomic writes** for CSV/JSON
  - **CSV formula guard**
  - **Valid JSON** under all numeric inputs
- **Numerics**
  - Non-finite values filtered/treated as missing
  - Safe sorting; integer year ordering for plots
- **Rendering**
  - Embedded font registration avoids “FontUnavailable” in headless/CI

---

## Testing

Run all tests:

```bash
# use without --feature online to test local only
cargo test --features online
```

Coverage includes:

- CSV formula guard (prefix `'`)
- Output format logic in `cmd_get` (inference, explicit flags, conflicts)
- Basic numeric guards (non-finite handling)

---

## Contributing

Contributions are welcome! Please follow these guidelines:

1. **Discuss major changes first**: open an issue to align on scope/design before large features or public API changes.
2. **Keep PRs focused**: small, single-purpose PRs are easier to review and merge.
3. **Code quality**: ensure all of the following pass locally or supply arguments why some parts do not pass:

   ```bash
   cargo fmt --all
   cargo clippy --all-targets --all-features -- -D warnings
   cargo test --all -- --nocapture
   ```

4. **Commit messages**: using [Conventional Commits](https://www.conventionalcommits.org/) is appreciated (e.g., `feat:`, `fix:`, `docs:`).
5. **License**: by contributing, you agree your changes are dual MIT and Apache-2.-licensed.

---

## License

Licensed under either of

- [Apache License, Version 2.0](https://www.apache.org/licenses/LICENSE-2.0.txt)
- [MIT license](LICENSE)

at your option.

Any contribution intentionally submitted for inclusion in this work shall be
dual licensed as above, without any additional terms or conditions.

## Disclaimer

This project is an independent, community-maintained library and CLI. It is not affiliated with, endorsed by, or sponsored by The World Bank Group or any of its institutions. “The World Bank” name and any related trademarks are the property of The World Bank Group and are used here solely for identification purposes.

This software accesses publicly available data via the World Bank Indicators API. Your use of any World Bank data is governed by the World Bank’s terms and applicable data licenses. No warranty is provided; use at your own risk.

- World Bank Indicators API: <https://datahelpdesk.worldbank.org/knowledgebase/articles/889392>
- World Bank Terms of Use: <https://www.worldbank.org/terms>
- World Bank Data Terms (incl. licensing): <https://datacatalog.worldbank.org/terms-and-conditions>
