
# World Bank Data Rust 🦀📊📈

<p align="center">
  <img src="https://github.com/ArdentEmpiricist/world_bank_data_rust/blob/ef373fcdc6df6fd731400b81ace3af62795edea7/assets/logo.png?raw=true" alt="enc_file Logo" width="200"/>
</p>

A Rust **library + CLI** to fetch, store, visualize, and summarize [World Bank](https://datahelpdesk.worldbank.org/knowledgebase/topics/125589-developer-information) indicator data.

- **Library**: typed API client, tidy data model, summary stats, flexible plotting
- **CLI**: one-liners to fetch → save/plot → print summaries

## Install

```bash
# From source (inside the project folder)
cargo install --path .
```

---

## Quickstart

```bash
world_bank_data_rust get \
  --countries DEU,USA \
  --indicators SP.POP.TOTL \
  --date 2010:2020 \
  --out pop.csv --format csv \
  --plot pop.svg \
  --stats \
  --locale de
```

- Saves tidy CSV to `pop.csv`
- Creates a chart at `pop.svg`
- Prints grouped stats to stdout with **German** number formatting

---

## Full CLI Reference

### Command

```
world_bank_data_rust get [OPTIONS]
```

### Required

| Flag | Type | Description |
|---|---|---|
| `-c, --countries` | string | Comma/semicolon-separated country/region codes. Examples: `DEU,USA`, `EUU`. |
| `-i, --indicators` | string | Comma/semicolon-separated indicator codes. Example: `SP.POP.TOTL`. |

### Optional

| Flag | Type | Default | Description |
|---|---|---:|---|
| `-d, --date` | `YYYY` or `YYYY:YYYY` | — | Single year or inclusive range. Example: `2010:2020`. |
| `--source` | integer | — | Indicator source id (e.g., `2` for WDI). Required by API **when querying multiple indicators**. |
| `--out` | path | — | Save results to file. Use with `--format` or let extension infer format. |
| `--format` | enum `csv,json` | — | Output format for `--out`. |
| `--plot` | path | — | Create a chart at the given path (`.svg` or `.png`). |
| `--width` | integer | `1000` | Chart width (pixels). |
| `--height` | integer | `600` | Chart height (pixels). |
| `--title` | string | `"World Bank Indicator(s)"` | Chart title. |
| `--stats` | flag | `false` | Print grouped summary stats to stdout. |
| `--locale` | string | `"en"` | Number formatting for chart labels & stats. Supports `en,de,fr,es,it,pt,nl,…`. |
| `--legend` | enum `inside,right,top,bottom` | `right` | Where the legend is rendered. `inside` overlays the plot; others avoid overlap. |
| `--plot-kind` | enum `line,scatter,line-points,area,stacked-area,grouped-bar,loess` | `line` | Chart type (see examples below). |
| `--loess-span` | float in `(0,1]` | `0.3` | Smoothing span for `--plot-kind loess`. Ignored for other kinds. |

### Notes

- `--plot` supports **SVG** and **PNG** based on the file extension.
- For **multiple indicators** in a single request, the World Bank API requires a `--source` id (commonly `2` for WDI).
- Legends: `right`/`top`/`bottom` render outside the plot to avoid overlap; long labels wrap/truncate as needed; markers are centered with text.
- Colors follow the **Microsoft Office palette** (Blue, Orange, Gray, Gold, Light Blue, Green, …).

---

## Examples

### 1) Fetch & Save

**CSV**

```bash
world_bank_data_rust get \
  --countries DEU,USA \
  --indicators SP.POP.TOTL \
  --date 2010:2020 \
  --out pop.csv --format csv
```

**JSON (format via extension)**

```bash
world_bank_data_rust get \
  --countries DEU \
  --indicators SP.POP.TOTL \
  --date 2019 \
  --out pop.json
```

### 2) Basic Plots

**Line (default)**

```bash
world_bank_data_rust get \
  -c DEU,USA -i SP.POP.TOTL -d 2010:2020 \
  --plot pop.svg --legend right
```

**Scatter**

```bash
world_bank_data_rust get \
  -c DEU,USA -i SP.POP.TOTL -d 2010:2020 \
  --plot pop_scatter.svg --legend top \
  --plot-kind scatter
```

**Line + Points**

```bash
world_bank_data_rust get \
  -c DEU,USA -i SP.POP.TOTL -d 2010:2020 \
  --plot pop_lp.svg --legend bottom \
  --plot-kind line-points
```

**Area**

```bash
world_bank_data_rust get \
  -c DEU,USA -i SP.POP.TOTL -d 2010:2020 \
  --plot pop_area.svg --legend right \
  --plot-kind area
```

### 3) Advanced Plots

**Stacked Area** (stacks **positive** contributions per year)

```bash
world_bank_data_rust get \
  -c DEU,USA \
  -i SP.POP.TOTL \
  -d 2010:2020 \
  --plot pop_stacked.svg \
  --legend bottom \
  --plot-kind stacked-area
```

**Grouped Bar** (bars offset within each year)

```bash
world_bank_data_rust get \
  -c DEU,USA \
  -i SP.POP.TOTL \
  -d 2010:2020 \
  --plot pop_bars.svg \
  --legend top \
  --plot-kind grouped-bar
```

**LOESS Smoothing** (local regression; `span` controls smoothness)

```bash
world_bank_data_rust get \
  -c DEU,USA \
  -i SP.POP.TOTL \
  -d 1960:2020 \
  --plot pop_loess.svg \
  --legend right \
  --plot-kind loess \
  --loess-span 0.25
```

### 4) International Formatting & Titles

**German formatting + custom title**

```bash
world_bank_data_rust get \
  -c DEU,USA \
  -i SP.POP.TOTL \
  -d 2010:2020 \
  --plot pop_de.svg \
  --legend right \
  --locale de \
  --title "Bevölkerung gesamt (2010–2020)"
```

**PNG output**

```bash
world_bank_data_rust get \
  -c DEU,USA \
  -i SP.POP.TOTL \
  -d 2010:2020 \
  --plot pop.png \
  --legend right
```

### 5) Stats in the Terminal

```bash
world_bank_data_rust get \
  -c DEU,USA \
  -i SP.POP.TOTL \
  -d 2010:2020 \
  --stats --locale en
```

Output looks like:

```
DEU • SP.POP.TOTL  count=11 missing=0  min=80,274,983 max=83,160,871 mean=81,814,340 median=81,776,930
USA • SP.POP.TOTL  count=11 missing=0  min=...       max=...       mean=...       median=...
```

---

## Library Highlights (Rust)

```rust
use world_bank_data_rust::{Client, DateSpec};
use world_bank_data_rust::viz::{self, LegendMode, PlotKind};

let client = Client::default();
let data = client.fetch(
    &["DEU".into(), "USA".into()],
    &["SP.POP.TOTL".into()],
    Some(DateSpec::Range{ start: 2010, end: 2020 }),
    None,
)?;

// Save
world_bank_data_rust::storage::save_csv(&data, "pop.csv")?;

// Plot (scatter, German labels)
viz::plot_chart(&data, "pop.svg", 1000, 600, "de", LegendMode::Top, "Population (2010–2020)", PlotKind::Scatter, 0.3)?;

// Summaries
let summaries = world_bank_data_rust::stats::grouped_summary(&data);
for s in summaries {
    println!("{:?}", s);
}
# Ok::<(), anyhow::Error>(())
```

---

## Tips & Notes

- **Legends:** Use `--legend top` / `bottom` for long labels (they wrap over multiple rows). `--legend right` truncates long labels to fit the panel.
- **Inside legend:** `--legend inside` overlays the plot and can overlap lines; keep for quick previews.
- **Multiple indicators:** Pass `--source 2` (WDI) when using multiple indicator codes; otherwise the API may error.
- **Colors:** Series colors follow the Microsoft Office palette (Blue, Orange, Gray, Gold, Light Blue, Green, …).
- **Axes:** Y-axis shows whole numbers with locale-specific thousands separators.

## License

Dual-licensed under either **MIT** or **Apache-2.0**.
