
[![Crates.io](https://img.shields.io/crates/v/world_bank_data_rust?label=Crates.io)](https://crates.io/crates/world_bank_data_rust)
[![rust-clippy analyze](https://img.shields.io/github/actions/workflow/status/ardentempiricist/world_bank_data_rust/rust-clippy.yml?label=Rust%20Clippy)](https://github.com/ArdentEmpiricist/world_bank_data_rust/actions/workflows/rust-clippy.yml)
[![Deploy](https://github.com/ArdentEmpiricist/world_bank_data_rust/actions/workflows/deploy.yml/badge.svg)](https://github.com/ArdentEmpiricist/world_bank_data_rust/actions/workflows/deploy.yml)
[![Documentation](https://docs.rs/world_bank_data_rust/badge.svg)](https://docs.rs/world_bank_data_rust/)
[![Crates.io](https://img.shields.io/crates/d/world_bank_data_rust?color=darkblue&label=Downloads)](https://crates.io/crates/world_bank_data_rust)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

# World Bank Data Rust 🦀📊📈

<p align="center">
  <img src="https://github.com/ArdentEmpiricist/world_bank_data_rust/blob/ef373fcdc6df6fd731400b81ace3af62795edea7/assets/logo.png?raw=true" alt="world_bank_data_rust Logo" width="200"/>
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

---

## Features

### What it can do

- **Retrieve data** from the World Bank by country/countries, indicator(s), and optional date range.
- **Show short stats in the terminal** (grouped min / max / mean / median per (indicator, country)).
- **Export datasets** to **CSV** or **JSON** (format inferred from `--out` extension or set via `--format`).  
  Exports are **atomic** and CSV is **spreadsheet-safe**.
- **Export plots** as **SVG** or **PNG** (backend inferred from `--plot` file extension).

### Under the hood

- **Hardened HTTPS client** (rustls, connect/request timeouts, limited redirects, descriptive `User-Agent`: `world_bank_data_rust`).
- **Robust URL handling**
- **Transient error resilience** (small retry/backoff) and **page caps**.
- **Valid JSON** under all inputs (`NaN`/`±∞` → `null`).
- **Numerical stability** (non-finite values treated as missing; safe float sorting).
- **Portable plotting** with an embedded TTF font for CI/headless environments.

---

## Install

Download a prebuilt binary (GitHub Releases):

1) Go to **GitHub → Releases**: [https://github.com/ArdentEmpiricist/world_bank_data_rust/releases/new](https://github.com/ArdentEmpiricist/world_bank_data_rust/releases/new)
2) Download the asset for your platform

Via cargo/crates.io:

```bash
cargo install world_bank_data_rust
world_bank_data_rust --help
```

From source:

```bash
git clone <this-repo>
cd world_bank_data_rust
cargo build --release
```

As a library (example):

```toml
[dependencies]
world_bank_data_rust = { path = "." } # replace with git or registry when published
anyhow = "1"
```

---

## Quick start (CLI)

Fetch population for Germany & France (2000–2023) and write to CSV (format inferred from extension):

```bash
world_bank_data_rust get \
  --countries DEU;FRA \
  --indicators SP.POP.TOTL \
  --date 2000:2023 \
  --out pop.csv
```

Render a plot (backend inferred from extension):

```bash
world_bank_data_rust get \
  --countries DEU;FRA \
  --indicators SP.POP.TOTL \
  --date 2000:2023 \
  --plot pop.svg
```

Example for:

```bash
world_bank_data_rust get \
  --countries USA,CHN,DEU,IND \
  --indicators NY.GDP.MKTP.CD \
  --date 1970:2025 \
  --plot pop.svg
  --plot-kind line-points
```

<p align="center">
  <img src="https://raw.githubusercontent.com/ArdentEmpiricist/world_bank_data_rust/f35a19f1f333c5d88e2299073fba367ef56880e7/assets/example_plot.svg?raw=true" alt="enc_file Logo" style='width: 100%; object-fit: contain'/>
</p>

---

## CLI usage

Subcommand `get` accepts at least:

- `--countries` ISO2/ISO3 codes separated by `;` or `,` (e.g., `DEU;FRA`)
- `--indicators` World Bank indicator IDs (e.g., `SP.POP.TOTL`)
- `--date` optional year or range (e.g., `2020` or `2000:2023`)
- `--out <PATH>` optional export (CSV/JSON); **atomic**
- `--plot <PATH>` optional chart output (SVG/PNG), using Plotters

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
world_bank_data_rust get --countries DEU --indicators SP.POP.TOTL --out data.csv

# JSON via extension inference
world_bank_data_rust get --countries DEU --indicators SP.POP.TOTL --out data.json

# Unknown extension allowed when --format is explicit
world_bank_data_rust get --countries DEU --indicators SP.POP.TOTL --out dump.xyz --format csv

# Error on conflict
world_bank_data_rust get --countries DEU --indicators SP.POP.TOTL --out data.csv --format json
```

---

## Library usage

The crate exposes modules for API access, models, storage, statistics, and plotting.

### Add to `Cargo.toml`

```toml
[dependencies]
world_bank_data_rust = { path = "." } # change to your source
anyhow = "1"
```

### Fetch data

```rust
use anyhow::Result;
use world_bank_data_rust::api::Client;
use world_bank_data_rust::models::DateSpec;

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

### Export data (atomic CSV/JSON)

```rust
use world_bank_data_rust::storage::{save_csv, save_json};

save_csv(&points, "pop.csv")?;   // spreadsheet-safe + atomic
save_json(&points, "pop.json")?; // non-finite -> null + atomic
```

- **CSV**: fixed header order; cells beginning with `=`, `+`, `-`, `@` are prefixed with `'`.
- **JSON**: pretty-printed; non-finite floats are serialized as `null`.

Both writers use a tempfile in the destination directory and atomically replace the target file.

### Compute grouped summaries

```rust
use world_bank_data_rust::stats::{grouped_summary, Summary};

let summaries: Vec<Summary> = grouped_summary(&points);
// Summary contains: key (indicator_id, country_iso3), count, missing, min, max, mean, median.
// Non-finite values are counted as missing; sorting avoids panics on floats.
```

### Plot charts

```rust
use world_bank_data_rust::viz::plot_chart;

// `plot_chart` filters non-finite values and sorts by integer year.
// The backend is selected from the output extension (.svg, .png).
plot_chart(&points, "pop.svg")?;
```

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
