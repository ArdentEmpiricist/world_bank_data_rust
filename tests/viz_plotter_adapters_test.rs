use plotters::prelude::*;
use plotters_bitmap::BitMapBackend;
use world_bank_data_rust::viz_plotters_adapter::{fill_style, line_style, make_marker, rgb_color};
use world_bank_data_rust::viz_style::SeriesStyle;

#[test]
fn draw_markers_and_lines_ok() -> Result<(), Box<dyn std::error::Error>> {
    // Output under target/, which is ignored by Git
    let root = BitMapBackend::new("target/test_marker.png", (480, 320)).into_drawing_area();
    root.fill(&WHITE)?;

    // Build a simple integer chart (i32 axes)
    let mut chart = ChartBuilder::on(&root)
        .margin(10)
        .caption("viz_plotters_adapter smoke test", ("sans-serif", 18))
        .build_cartesian_2d(0..10, 0..10)?;

    chart.configure_mesh().draw()?;

    // Sample points/rect
    let points: Vec<(i32, i32)> = vec![(1, 1), (3, 5), (6, 4), (9, 8)];
    let bar = ((2, 0), (4, 6));

    // A deterministic style
    let style = SeriesStyle::for_series("USA", "SP.POP.TOTL");

    // Lines
    chart
        .draw_series(LineSeries::new(points.clone(), line_style(&style)))?
        .label("USA — SP.POP.TOTL");

    // Markers along the same series (note we pass ShapeStyle to PointSeries)
    chart.draw_series(PointSeries::of_element(
        points.clone(),
        style.marker_size as i32,
        fill_style(&style),
        &|c, s, st| make_marker::<BitMapBackend>(c, s, st.clone(), style.marker),
    ))?;

    // Bar example
    chart.draw_series(std::iter::once(Rectangle::new(
        [bar.0, bar.1],
        fill_style(&style),
    )))?;

    root.present()?;
    Ok(())
}
