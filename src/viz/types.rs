//! Public types and constants for the visualization module.

/// Legend placement options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegendMode {
    /// Overlay legend inside the plotting area (may overlap data).
    Inside,
    /// Separate, non-overlapping legend panel on the right side.
    Right,
    /// Separate, non-overlapping legend band at the top.
    Top,
    /// Separate, non-overlapping legend band at the bottom.
    Bottom,
}

/// Plot types supported by this module.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlotKind {
    /// Multi-series line chart (default).
    Line,
    /// Scatter (markers only).
    Scatter,
    /// Line + markers overlay.
    LinePoints,
    /// Area chart (filled area from baseline to values).
    Area,
    /// Stacked area chart (positive values stacked upward).
    StackedArea,
    /// Grouped bar chart (per year, bars per series).
    GroupedBar,
    /// LOESS smoothed line (span parameter controls smoothness).
    Loess,
}

/// Default legend placement following mainstream design guidance:
/// - Horizontal legend **below** the chart works well for dashboards and keeps labels close
///   to the x-axis start.
///   References: IBM Carbon (bottom/top as default), U.S. Gov Data Viz Standards.
///   (You can still override per call.)
pub const DEFAULT_LEGEND_MODE: LegendMode = LegendMode::Bottom;
