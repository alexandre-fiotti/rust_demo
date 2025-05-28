use chrono::NaiveDate;
use plotters::prelude::*;
use super::data_processing::{
    ProcessedMultiRepoData, TimeAxis, MetricType
};

/// Chart configuration options
#[derive(Debug, Clone)]
pub struct ChartConfig {
    pub width: u32,
    pub height: u32,
    pub title: String,
    pub show_legend: bool,
    pub colors: Vec<RGBColor>,
}

impl Default for ChartConfig {
    fn default() -> Self {
        Self {
            width: 800,
            height: 400,
            title: "Repository Star Metrics".to_string(),
            show_legend: true,
            colors: vec![
                BLUE,
                RED,
                GREEN,
                MAGENTA,
                CYAN,
                RGBColor(255, 165, 0), // Orange
                RGBColor(128, 0, 128), // Purple
                RGBColor(255, 192, 203), // Pink
            ],
        }
    }
}

/// Generates an SVG chart for multiple repositories with the specified metric type
pub fn generate_multi_repo_chart(
    data: &ProcessedMultiRepoData,
    config: &ChartConfig,
) -> Result<String, String> {
    if data.repositories.is_empty() {
        return Ok(generate_empty_chart(&config.title, config.width, config.height));
    }

    let mut buffer = String::new();
    {
        let root = SVGBackend::with_string(&mut buffer, (config.width, config.height))
            .into_drawing_area();
        root.fill(&WHITE)
            .map_err(|e| format!("Failed to fill background: {}", e))?;

        match &data.time_axis {
            TimeAxis::Absolute { min_date, max_date } => {
                generate_absolute_chart(&root, data, config, *min_date, *max_date)?;
            }
            TimeAxis::Relative { max_days, start_date } => {
                generate_relative_chart(&root, data, config, *max_days, *start_date)?;
            }
        }

        root.present()
            .map_err(|e| format!("Failed to present chart: {}", e))?;
    }

    Ok(buffer)
}

/// Generates a chart with absolute time axis (actual dates)
fn generate_absolute_chart(
    root: &DrawingArea<SVGBackend, plotters::coord::Shift>,
    data: &ProcessedMultiRepoData,
    config: &ChartConfig,
    min_date: NaiveDate,
    max_date: NaiveDate,
) -> Result<(), String> {
    let (y_min, y_max) = calculate_y_range(data)?;
    let y_desc = get_y_axis_description(&data.metric_type);

    let mut chart = ChartBuilder::on(root)
        .caption(&config.title, ("Arial", 24).into_font())
        .margin(20)
        .x_label_area_size(50)
        .y_label_area_size(60)
        .build_cartesian_2d(min_date..max_date, y_min..y_max)
        .map_err(|e| format!("Failed to build chart: {}", e))?;

    chart
        .configure_mesh()
        .x_desc("Date")
        .y_desc(&y_desc)
        .x_label_formatter(&|date| date.format("%m/%d").to_string())
        .y_label_formatter(&|y| format_y_value(*y, &data.metric_type))
        .draw()
        .map_err(|e| format!("Failed to configure mesh: {}", e))?;

    // Draw data for each repository
    for (repo_idx, repo) in data.repositories.iter().enumerate() {
        let color = config.colors.get(repo_idx % config.colors.len()).unwrap_or(&BLUE);
        let label = format!("{}/{}", repo.owner, repo.name);

        // Draw line series
        chart
            .draw_series(LineSeries::new(
                repo.data_points.iter().map(|point| (point.date, point.value)),
                color.stroke_width(2),
            ))
            .map_err(|e| format!("Failed to draw line series for {}: {}", label, e))?
            .label(&label)
            .legend(move |(x, y)| PathElement::new(vec![(x, y), (x + 10, y)], *color));

        // Note: Removed individual point circles for cleaner appearance
    }

    if config.show_legend {
        chart
            .configure_series_labels()
            .background_style(&WHITE.mix(0.8))
            .border_style(&BLACK)
            .draw()
            .map_err(|e| format!("Failed to draw legend: {}", e))?;
    }

    Ok(())
}

/// Generates a chart with relative time axis (days/months/years from start)
fn generate_relative_chart(
    root: &DrawingArea<SVGBackend, plotters::coord::Shift>,
    data: &ProcessedMultiRepoData,
    config: &ChartConfig,
    max_days: i64,
    _start_date: NaiveDate,
) -> Result<(), String> {
    let (y_min, y_max) = calculate_y_range(data)?;
    let y_desc = get_y_axis_description(&data.metric_type);

    let mut chart = ChartBuilder::on(root)
        .caption(&config.title, ("Arial", 24).into_font())
        .margin(20)
        .x_label_area_size(50)
        .y_label_area_size(60)
        .build_cartesian_2d(0f64..(max_days as f64 / 365.25), y_min..y_max)
        .map_err(|e| format!("Failed to build chart: {}", e))?;

    chart
        .configure_mesh()
        .x_desc("Time Since Start (years)")
        .y_desc(&y_desc)
        .y_label_formatter(&|y| format_y_value(*y, &data.metric_type))
        .draw()
        .map_err(|e| format!("Failed to configure mesh: {}", e))?;

    // Find the earliest date to convert to relative days
    let earliest_date = data
        .repositories
        .iter()
        .flat_map(|repo| &repo.data_points)
        .map(|point| point.date)
        .min()
        .ok_or("No data points found")?;

    // Draw data for each repository
    for (repo_idx, repo) in data.repositories.iter().enumerate() {
        let color = config.colors.get(repo_idx % config.colors.len()).unwrap_or(&BLUE);
        let label = format!("{}/{}", repo.owner, repo.name);

        // Use relative_days if available, otherwise calculate from dates
        let relative_points: Vec<(f64, f64)> = repo
            .data_points
            .iter()
            .map(|point| {
                let days = if let Some(relative_days) = point.relative_days {
                    relative_days
                } else {
                    point.date.signed_duration_since(earliest_date).num_days()
                };
                let years = days as f64 / 365.25;
                (years, point.value)
            })
            .collect();

        // Draw line series
        chart
            .draw_series(LineSeries::new(relative_points.iter().cloned(), color.stroke_width(2)))
            .map_err(|e| format!("Failed to draw line series for {}: {}", label, e))?
            .label(&label)
            .legend(move |(x, y)| PathElement::new(vec![(x, y), (x + 10, y)], *color));

        // Note: Removed individual point circles for cleaner appearance
    }

    if config.show_legend {
        chart
            .configure_series_labels()
            .background_style(&WHITE.mix(0.8))
            .border_style(&BLACK)
            .draw()
            .map_err(|e| format!("Failed to draw legend: {}", e))?;
    }

    Ok(())
}

/// Calculates the Y-axis range for the chart
fn calculate_y_range(data: &ProcessedMultiRepoData) -> Result<(f64, f64), String> {
    let all_values: Vec<f64> = data
        .repositories
        .iter()
        .flat_map(|repo| &repo.data_points)
        .map(|point| point.value)
        .collect();

    if all_values.is_empty() {
        return Ok((0.0, 10.0));
    }

    let min_val = all_values
        .iter()
        .fold(f64::INFINITY, |a, &b| a.min(b));
    let max_val = all_values
        .iter()
        .fold(f64::NEG_INFINITY, |a, &b| a.max(b));

    // Add some padding
    let padding = (max_val - min_val) * 0.1;
    
    // Only enforce minimum of 0 for position metrics (cumulative star counts)
    // Speed and acceleration can legitimately be negative
    let y_min = match data.metric_type {
        MetricType::Position => (min_val - padding).max(0.0), // Don't go below 0 for star counts
        MetricType::Speed | MetricType::Acceleration => min_val - padding, // Allow negative values
    };
    let y_max = max_val + padding;

    Ok((y_min, y_max))
}

/// Gets the Y-axis description based on metric type
fn get_y_axis_description(metric_type: &MetricType) -> String {
    match metric_type {
        MetricType::Position => "Total Stars".to_string(),
        MetricType::Speed => "Daily Stars".to_string(),
        MetricType::Acceleration => "Star Acceleration".to_string(),
    }
}

/// Formats Y-axis values based on metric type
fn format_y_value(value: f64, metric_type: &MetricType) -> String {
    match metric_type {
        MetricType::Position => {
            if value >= 1_000_000.0 {
                format!("{:.1}M", value / 1_000_000.0)
            } else if value >= 1_000.0 {
                format!("{:.1}K", value / 1_000.0)
            } else {
                format!("{:.0}", value)
            }
        }
        MetricType::Speed | MetricType::Acceleration => {
            if value.abs() >= 1_000.0 {
                format!("{:.1}K", value / 1_000.0)
            } else {
                format!("{:.0}", value)
            }
        }
    }
}

/// Generates an empty chart when no data is available
fn generate_empty_chart(title: &str, width: u32, height: u32) -> String {
    format!(
        "<svg width=\"{}\" height=\"{}\" xmlns=\"http://www.w3.org/2000/svg\">\
            <rect width=\"100%\" height=\"100%\" fill=\"white\"/>\
            <text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"Arial\" font-size=\"18\" fill=\"#666666\">\
                No data available for: {}\
            </text>\
        </svg>",
        width,
        height,
        width / 2,
        height / 2,
        title
    )
} 