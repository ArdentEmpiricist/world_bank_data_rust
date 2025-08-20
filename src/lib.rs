//! # wbi_rs
//!
//! A lightweight **Rust library + CLI** to fetch, store, visualize, and summarize
//! [World Bank Indicators API](https://datahelpdesk.worldbank.org/knowledgebase/articles/889392-about-the-indicators-api-documentation)
//! data.
//!
//! ## Highlights
//! - Synchronous API client (`api::Client`)
//! - Tidy data model (`models::DataPoint`)
//! - Summary stats (`stats::grouped_summary`)
//! - CSV/JSON export (`storage`)
//! - SVG/PNG charts (`viz`) with legend placement, locale formatting, and multiple plot types
//!
//! ## Feature flags
//! - `online`: enables live API tests/examples. (The library itself works without it.)
//!
//! Country-consistent styling is available as a runtime option via `viz::plot_chart(.., Some(true))` or the CLI `--country-styles` flag.
//!
//! ## Quick example
//! ```no_run
//! use wbi_rs::{Client, DateSpec};
//! use wbi_rs::viz::{LegendMode, PlotKind};
//!
//! // 1) Fetch observations
//! let client = Client::default();
//! let data = client.fetch(
//!     &["DEU".into(), "USA".into()],
//!     &["SP.POP.TOTL".into()],
//!     Some(DateSpec::Range { start: 2010, end: 2020 }),
//!     None,
//! )?;
//!
//! // 2) Plot to SVG (line chart, legend on the right, English locale)
//! wbi_rs::viz::plot_chart(
//!     &data,
//!     "pop.svg",
//!     1000,
//!     600,
//!     "en",
//!     LegendMode::Right,
//!     "Population (2010–2020)",
//!     PlotKind::Line,
//!     0.3, // loess_span (ignored unless PlotKind::Loess)
//!     None, // no country styles in tests
//! )?;
//!
//! // 3) Print grouped summary stats
//! let summaries = wbi_rs::stats::grouped_summary(&data);
//! for s in summaries {
//!     println!("{:?}", s);
//! }
//! # Ok::<(), anyhow::Error>(())
//! ```

pub mod api;
pub mod models;
pub mod stats;
pub mod storage;
pub mod viz;
pub mod viz_plotters_adapter;
pub mod viz_style;

// Country-consistent styling module (always available)
pub mod style;

pub use api::Client;
pub use models::{DataPoint, DateSpec, GroupKey};
