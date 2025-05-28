use chrono::{NaiveDate, Duration};
use std::collections::HashMap;

/// Represents different types of metrics that can be calculated from star data
#[derive(Debug, Clone, PartialEq)]
pub enum MetricType {
    Position,     // Cumulative star count
    Speed,        // Daily star count (first derivative)
    Acceleration, // Change in daily star count (second derivative)
}

/// Represents a single repository's processed data
#[derive(Debug, Clone)]
pub struct RepositoryData {
    pub owner: String,
    pub name: String,
    pub data_points: Vec<DataPoint>,
}

/// A single data point with date and value
#[derive(Debug, Clone)]
pub struct DataPoint {
    pub date: NaiveDate,
    pub value: f64,
    pub relative_days: Option<i64>, // Store relative days for relative charts
}

/// Processed data for multiple repositories with normalized time axis
#[derive(Debug)]
pub struct ProcessedMultiRepoData {
    pub repositories: Vec<RepositoryData>,
    pub time_axis: TimeAxis,
    pub metric_type: MetricType,
    pub start_date: Option<NaiveDate>, // Store start date for accurate label calculations
}

/// Time axis configuration
#[derive(Debug, Clone)]
pub enum TimeAxis {
    Absolute {
        min_date: NaiveDate,
        max_date: NaiveDate,
    },
    Relative {
        max_days: i64,
        start_date: NaiveDate, // Store start date for accurate calculations
    },
}

/// Processes raw star data for multiple repositories
pub fn process_multi_repo_data(
    repo_data: Vec<(String, String, Vec<(NaiveDate, i64)>)>, // (owner, name, daily_counts)
    metric_types: &[MetricType],
    relative_x_axis: bool,
) -> Result<Vec<ProcessedMultiRepoData>, String> {
    if repo_data.is_empty() {
        return Ok(Vec::new());
    }

    let mut results = Vec::new();

    for metric_type in metric_types {
        let mut processed_repos = Vec::new();

        // Find the earliest start date across all repositories for relative mode
        let earliest_date = if relative_x_axis {
            repo_data
                .iter()
                .filter_map(|(_, _, data)| data.first().map(|(date, _)| *date))
                .min()
        } else {
            None
        };

        for (owner, name, daily_counts) in &repo_data {
            let processed_data = process_single_repo_data(daily_counts, metric_type, earliest_date)?;
            processed_repos.push(RepositoryData {
                owner: owner.clone(),
                name: name.clone(),
                data_points: processed_data,
            });
        }

        let time_axis = calculate_time_axis(&processed_repos, relative_x_axis)?;

        results.push(ProcessedMultiRepoData {
            repositories: processed_repos,
            time_axis,
            metric_type: metric_type.clone(),
            start_date: earliest_date,
        });
    }

    Ok(results)
}

/// Processes data for a single repository based on the metric type
fn process_single_repo_data(
    daily_counts: &[(NaiveDate, i64)],
    metric_type: &MetricType,
    relative_start_date: Option<NaiveDate>,
) -> Result<Vec<DataPoint>, String> {
    if daily_counts.is_empty() {
        return Ok(Vec::new());
    }

    let data_points = match metric_type {
        MetricType::Position => calculate_position_data(daily_counts),
        MetricType::Speed => calculate_speed_data(daily_counts),
        MetricType::Acceleration => calculate_acceleration_data(daily_counts),
    };

    // Apply relative time transformation if needed
    if let Some(start_date) = relative_start_date {
        Ok(apply_relative_time_transformation(data_points, start_date))
    } else {
        Ok(data_points)
    }
}

/// Calculates cumulative star counts (position)
fn calculate_position_data(daily_counts: &[(NaiveDate, i64)]) -> Vec<DataPoint> {
    let mut cumulative = 0i64;
    daily_counts
        .iter()
        .map(|(date, count)| {
            cumulative += count;
            DataPoint {
                date: *date,
                value: cumulative as f64,
                relative_days: None,
            }
        })
        .collect()
}

/// Calculates daily star counts (speed/first derivative)
fn calculate_speed_data(daily_counts: &[(NaiveDate, i64)]) -> Vec<DataPoint> {
    // Fill in missing days with 0 values for accurate speed calculation
    let filled_data = fill_missing_days(daily_counts);
    
    filled_data
        .iter()
        .map(|(date, count)| DataPoint {
            date: *date,
            value: *count as f64,
            relative_days: None,
        })
        .collect()
}

/// Calculates acceleration (second derivative of position)
fn calculate_acceleration_data(daily_counts: &[(NaiveDate, i64)]) -> Vec<DataPoint> {
    // Fill in missing days with 0 values for accurate acceleration calculation
    let filled_data = fill_missing_days(daily_counts);
    
    if filled_data.len() < 2 {
        return filled_data
            .iter()
            .map(|(date, _)| DataPoint {
                date: *date,
                value: 0.0,
                relative_days: None,
            })
            .collect();
    }

    let mut result = Vec::new();
    
    // First point has acceleration of 0
    result.push(DataPoint {
        date: filled_data[0].0,
        value: 0.0,
        relative_days: None,
    });

    // Calculate acceleration as change in daily count
    for i in 1..filled_data.len() {
        let prev_count = filled_data[i - 1].1 as f64;
        let curr_count = filled_data[i].1 as f64;
        let acceleration = curr_count - prev_count;

        result.push(DataPoint {
            date: filled_data[i].0,
            value: acceleration,
            relative_days: None,
        });
    }

    result
}

/// Fills in missing days with 0 star counts
/// The database only returns days with actual stars, but for speed/acceleration
/// we need to include days with 0 stars for accurate calculations
fn fill_missing_days(daily_counts: &[(NaiveDate, i64)]) -> Vec<(NaiveDate, i64)> {
    if daily_counts.is_empty() {
        return Vec::new();
    }

    let start_date = daily_counts.first().unwrap().0;
    let end_date = daily_counts.last().unwrap().0;
    
    // Create a map for quick lookup of existing data
    let mut data_map: HashMap<NaiveDate, i64> = daily_counts.iter().cloned().collect();
    
    let mut result = Vec::new();
    let mut current_date = start_date;
    
    while current_date <= end_date {
        let count = data_map.remove(&current_date).unwrap_or(0);
        result.push((current_date, count));
        current_date = current_date.succ_opt().unwrap_or(current_date);
    }
    
    result
}

/// Applies relative time transformation to data points
fn apply_relative_time_transformation(
    data_points: Vec<DataPoint>,
    start_date: NaiveDate,
) -> Vec<DataPoint> {
    data_points
        .into_iter()
        .map(|point| {
            let days_from_start = point.date.signed_duration_since(start_date).num_days();
            DataPoint {
                date: start_date + Duration::days(days_from_start),
                value: point.value,
                relative_days: Some(days_from_start),
            }
        })
        .collect()
}

/// Calculates the appropriate time axis for the processed data
fn calculate_time_axis(
    repositories: &[RepositoryData],
    relative_x_axis: bool,
) -> Result<TimeAxis, String> {
    if repositories.is_empty() {
        return Err("No repository data provided".to_string());
    }

    if relative_x_axis {
        let max_days = repositories
            .iter()
            .flat_map(|repo| &repo.data_points)
            .map(|point| point.date)
            .max()
            .and_then(|max_date| {
                repositories
                    .iter()
                    .flat_map(|repo| &repo.data_points)
                    .map(|point| point.date)
                    .min()
                    .map(|min_date| max_date.signed_duration_since(min_date).num_days())
            })
            .unwrap_or(0);

        let start_date = repositories
            .iter()
            .flat_map(|repo| &repo.data_points)
            .map(|point| point.date)
            .min()
            .ok_or("No data points found")?;

        Ok(TimeAxis::Relative {
            max_days,
            start_date,
        })
    } else {
        let min_date = repositories
            .iter()
            .flat_map(|repo| &repo.data_points)
            .map(|point| point.date)
            .min()
            .ok_or("No data points found")?;

        let max_date = repositories
            .iter()
            .flat_map(|repo| &repo.data_points)
            .map(|point| point.date)
            .max()
            .ok_or("No data points found")?;

        Ok(TimeAxis::Absolute { min_date, max_date })
    }
}

/// Utility function to format relative time labels (simple approximation for backward compatibility)
pub fn format_relative_time_label(days: i64) -> String {
    if days < 30 {
        format!("{}d", days)
    } else if days < 365 {
        let months = days / 30;
        format!("{}m", months)
    } else {
        let years = days / 365;
        format!("{}y", years)
    }
} 