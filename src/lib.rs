//! world_bank_data_rust
//!
//! A lightweight Rust library for retrieving, storing, visualizing, and analyzing
//! World Bank indicator data. Pairs with the `world_bank_data_rust` CLI.
//!
//! ### Features
//! - Fetch indicators for one or more countries/regions and years or ranges
//! - Save as CSV or JSON in a tidy, analysis-friendly schema
//! - Quick summary statistics (min, max, mean, median)
//! - Generate SVG/PNG line charts from the data
//!
//! ### Example
//! ```no_run
//! use world_bank_data_rust::{Client, DateSpec};
//!
//! let client = Client::default();
//! let data = client.fetch(
//!     &["DEU".into(), "USA".into()],
//!     &["SP.POP.TOTL".into()],
//!     Some(DateSpec::Range { start: 2010, end: 2020 }),
//!     None,
//! )?;
//! world_bank_data_rust::storage::save_csv(&data, "pop_2010_2020.csv")?;
//! world_bank_data_rust::viz::plot_lines(&data, "pop.svg", 1000, 600)?;
//! let stats = world_bank_data_rust::stats::grouped_summary(&data);
//! println!("{:#?}", stats);
//! # Ok::<(), anyhow::Error>(())
//! ```

pub mod api;
pub mod models;
pub mod stats;
pub mod storage;
pub mod viz;

pub use api::Client;
pub use models::{DateSpec, DataPoint, GroupKey};