# world_bank_data_rust 🦀📊

A Rust library + CLI to fetch, store, visualize, and summarize [World Bank](https://datahelpdesk.worldbank.org/knowledgebase/topics/125589-developer-information) indicator data.

- **Library**: typed API client, tidy data model, simple stats and plotting
- **CLI**: one-liner to fetch → save/plot → print summaries

## Install

```bash
# From source
git clone https://example.com/world_bank_data_rust
cd world_bank_data_rust
cargo install --path .
```

## Quickstart

```bash
world_bank_data_rust get \
  --countries DEU,USA \
  --indicators SP.POP.TOTL \
  --date 2010:2020 \
  --out pop.csv --format csv \
  --plot pop.svg \
  --stats \
  --locale de   # or en, fr, es, it, ...
```

This will:

1. Fetch population (SP.POP.TOTL) for Germany and the US from 2010–2020 (via the World Bank V2 API).
2. Save a tidy CSV (`indicator_id, country_iso3, year, value, ...`).
3. Render a multi-series line chart to `pop.svg` (axis labels formatted using the chosen locale).
4. Print grouped statistics (min/max/mean/median) to the terminal using the chosen locale.

## Data model

The CLI and library normalize API responses into **tidy rows**:

| field            | description                                      |
|------------------|--------------------------------------------------|
| indicator_id     | Indicator code (e.g., `SP.POP.TOTL`)             |
| indicator_name   | Human-readable indicator name                    |
| country_id       | Country/region id (often ISO2)                   |
| country_name     | Human-readable country/region name               |
| country_iso3     | ISO3 alpha-3 code                                |
| year             | Year as integer                                  |
| value            | Numeric observation (nullable)                   |
| unit             | Optional unit string                             |
| obs_status       | Optional observation status                      |
| decimal          | Decimal places                                   |

## API coverage

- Endpoint: `GET https://api.worldbank.org/v2/country/{codes}/indicator/{codes}`  
  - Response shape: JSON array `[Meta, [Entry, ...]]`  
  - Pagination fields in `Meta`: `page`, `pages`, `per_page`, `total`  
  - Parameters used: `format=json`, `date=YYYY` or `YYYY:YYYY`, `per_page=1000`, `page=N`  
  - Multiple `country` and `indicator` codes are separated by `;`  
  - For **multiple indicators**, the API requires a `source={id}` (e.g., `2` for WDI).

## Locale option

Use `--locale` to control number formatting in chart labels and printed stats.

```bash
# US formatting
world_bank_data_rust get -c DEU -i SP.POP.TOTL -d 2010:2020 --plot pop.svg --stats --locale en

# German formatting
world_bank_data_rust get -c DEU -i SP.POP.TOTL -d 2010:2020 --plot pop.svg --stats --locale de
```

## Testing

- **Unit tests** cover parsing, stats, storage, and plotting.
- **CLI smoke tests** validate help and (optionally) live fetches.

To run the online test (hits the live API), opt in:

```bash
cargo test --features online -- --ignored --test-threads=1
```

We force single-threaded execution only for the optional online test to avoid rate limiting under CI.

## Notes & Best Practices

- The client uses the **blocking** `reqwest` API to keep the CLI simple.
- We normalize values into a tidy schema to make downstream analysis easy.
- Plot rendering supports **SVG** and **PNG** (infer from file extension).
- For multiple indicators, pass `--source 2` to satisfy the API.
- Avoid exceedingly large queries; the CLI paginates with `per_page=1000`.

## License

Dual-licensed under either **MIT** or **Apache-2.0**.
