use axum::{
    extract::{Extension, Json},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    db::{
        repository::queries::get_repository_by_name,
        star::queries::get_daily_star_count,
        PgPool,
    },
    utils::{
        data_processing::{process_multi_repo_data, MetricType},
        chart::{generate_multi_repo_chart, ChartConfig},
    },
};

#[derive(Debug, Error)]
pub enum HandlerError {
    #[error("GetConnectionFromPool: {source}")]
    GetConnectionFromPool {
        #[from]
        source: r2d2::Error,
    },
    #[error("GetRepositoryByName: {source}")]
    GetRepositoryByName {
        #[from]
        source: crate::db::repository::queries::GetRepositoryByNameError,
    },
    #[error("RepositoryNotFound: {owner}/{name}")]
    RepositoryNotFound {
        owner: String,
        name: String,
    },
    #[error(transparent)]
    GetDailyStarCount{ 
        #[from] 
        source: crate::db::star::queries::GetDailyStarCountError 
    },
    #[error("DataProcessing: {message}")]
    DataProcessing {
        message: String,
    },
    #[error("ChartGeneration: {message}")]
    ChartGeneration {
        message: String,
    },
    #[error("InvalidRequest: {message}")]
    InvalidRequest {
        message: String,
    },
}

impl IntoResponse for HandlerError {
    fn into_response(self) -> axum::response::Response {
        match self {
            HandlerError::GetConnectionFromPool{ source } => (StatusCode::INTERNAL_SERVER_ERROR, source.to_string()).into_response(),
            HandlerError::GetRepositoryByName{ source } => (StatusCode::INTERNAL_SERVER_ERROR, source.to_string()).into_response(),
            HandlerError::RepositoryNotFound{ owner, name } => (StatusCode::NOT_FOUND, format!("Repository {owner}/{name} not found in database")).into_response(),
            HandlerError::GetDailyStarCount{ source } => (StatusCode::INTERNAL_SERVER_ERROR, source.to_string()).into_response(),
            HandlerError::DataProcessing{ message } => (StatusCode::INTERNAL_SERVER_ERROR, format!("Data processing failed: {message}")).into_response(),
            HandlerError::ChartGeneration{ message } => (StatusCode::INTERNAL_SERVER_ERROR, format!("Chart generation failed: {message}")).into_response(),
            HandlerError::InvalidRequest{ message } => (StatusCode::BAD_REQUEST, format!("Invalid request: {message}")).into_response(),
        }
    }
}

/// Repository specification in the request
#[derive(Debug, Deserialize)]
pub struct RepositorySpec {
    pub owner: String,
    pub name: String,
}

/// Request body for the enhanced endpoint
#[derive(Debug, Deserialize)]
pub struct RepoStarsReadDailyGraphRequestBody {
    /// List of repositories to include in the chart
    pub repositories: Vec<RepositorySpec>,
    
    /// Types of metrics to calculate and display
    /// Can include "position", "speed", "acceleration"
    #[serde(default = "default_metric_types")]
    pub metric_types: Vec<String>,
    
    /// Whether to use relative time axis (starting from 0)
    #[serde(default)]
    pub relative_x_axis: bool,
    
    /// Chart configuration options
    #[serde(default)]
    pub chart_config: Option<ChartConfigRequest>,
}

/// Chart configuration from request
#[derive(Debug, Deserialize)]
pub struct ChartConfigRequest {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub title: Option<String>,
    pub show_legend: Option<bool>,
}

fn default_metric_types() -> Vec<String> {
    vec!["position".to_string()]
}

/// Response structure for multiple charts
#[derive(Debug, Serialize)]
pub struct MultiChartResponse {
    pub charts: Vec<ChartResponse>,
}

/// Individual chart in the response
#[derive(Debug, Serialize)]
pub struct ChartResponse {
    pub metric_type: String,
    pub svg_content: String,
}

/// Axum handler: POST /github/repo_stars/read_daily_graph
pub async fn handler(
    Extension(pool): Extension<PgPool>,
    Json(input): Json<RepoStarsReadDailyGraphRequestBody>,
) -> impl IntoResponse {
    // Validate input
    if input.repositories.is_empty() {
        return HandlerError::InvalidRequest {
            message: "At least one repository must be specified".to_string(),
        }.into_response();
    }

    if input.repositories.len() > 10 {
        return HandlerError::InvalidRequest {
            message: "Maximum 10 repositories allowed per request".to_string(),
        }.into_response();
    }

    // Parse metric types
    let metric_types = match parse_metric_types(&input.metric_types) {
        Ok(types) => types,
        Err(err) => return HandlerError::InvalidRequest { message: err }.into_response(),
    };

    let mut conn = match pool.get() {
        Ok(c) => c,
        Err(source) => return HandlerError::GetConnectionFromPool { source }.into_response(),
    };

    // Fetch data for all repositories
    let mut repo_data = Vec::new();
    
    for repo_spec in &input.repositories {
        let repo = match get_repository_by_name(&mut conn, &repo_spec.owner, &repo_spec.name).await {
            Ok(Some(repo)) => repo,
            Ok(None) => {
                return HandlerError::RepositoryNotFound {
                    owner: repo_spec.owner.clone(),
                    name: repo_spec.name.clone(),
                }.into_response()
            }
            Err(source) => return HandlerError::GetRepositoryByName { source }.into_response(),
        };
        
        let star_counts = match get_daily_star_count(&mut conn, repo.id) {
            Ok(data) => data,
            Err(source) => return HandlerError::GetDailyStarCount { source }.into_response(),
        };

        repo_data.push((repo_spec.owner.clone(), repo_spec.name.clone(), star_counts));
    }

    // Process data for all metric types
    let processed_data = match process_multi_repo_data(repo_data, &metric_types, input.relative_x_axis) {
        Ok(data) => data,
        Err(message) => return HandlerError::DataProcessing { message }.into_response(),
    };

    // Generate charts for each metric type
    let mut chart_responses = Vec::new();
    
    for data in processed_data {
        let chart_config = build_chart_config(&input, &data.metric_type);
        
        match generate_multi_repo_chart(&data, &chart_config) {
            Ok(svg_content) => {
                let metric_type_name = match data.metric_type {
                    MetricType::Position => "position",
                    MetricType::Speed => "speed", 
                    MetricType::Acceleration => "acceleration",
                };
                chart_responses.push(ChartResponse {
                    metric_type: metric_type_name.to_string(),
                    svg_content,
                });
            },
            Err(message) => return HandlerError::ChartGeneration { message }.into_response(),
        }
    }

    // Return response based on number of charts
    if chart_responses.is_empty() {
        // No charts generated: return empty SVG
        let empty_svg = format!(
            "<svg width=\"800\" height=\"400\" xmlns=\"http://www.w3.org/2000/svg\">\
                <rect width=\"100%\" height=\"100%\" fill=\"white\"/>\
                <text x=\"400\" y=\"200\" text-anchor=\"middle\" font-family=\"Arial\" font-size=\"18\" fill=\"#666666\">\
                    No data available\
                </text>\
            </svg>"
        );
        Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "image/svg+xml")
            .header(header::CACHE_CONTROL, "public, max-age=3600")
            .body(empty_svg.into())
            .unwrap()
    } else if chart_responses.len() == 1 {
        // Single chart: return SVG directly
        let svg_content = chart_responses.into_iter().next().unwrap().svg_content;
        Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "image/svg+xml")
            .header(header::CACHE_CONTROL, "public, max-age=3600")
            .body(svg_content.into())
            .unwrap()
    } else {
        // Multiple charts: return JSON with array of SVGs
        let response = MultiChartResponse {
            charts: chart_responses,
        };
        Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::CACHE_CONTROL, "public, max-age=3600")
            .body(serde_json::to_string(&response).unwrap().into())
            .unwrap()
    }
}

/// Parses metric type strings into MetricType enum
fn parse_metric_types(metric_strings: &[String]) -> Result<Vec<MetricType>, String> {
    if metric_strings.is_empty() {
        return Ok(vec![MetricType::Position]);
    }

    let mut metric_types = Vec::new();
    
    for metric_str in metric_strings {
        let metric_type = match metric_str.to_lowercase().as_str() {
            "position" => MetricType::Position,
            "speed" => MetricType::Speed,
            "acceleration" => MetricType::Acceleration,
            _ => return Err(format!("Invalid metric type: '{}'. Valid types are: position, speed, acceleration", metric_str)),
        };
        
        if !metric_types.contains(&metric_type) {
            metric_types.push(metric_type);
        }
    }

    Ok(metric_types)
}

/// Builds chart configuration from request
fn build_chart_config(input: &RepoStarsReadDailyGraphRequestBody, metric_type: &MetricType) -> ChartConfig {
    let mut config = ChartConfig::default();
    
    if let Some(chart_config) = &input.chart_config {
        if let Some(width) = chart_config.width {
            config.width = width;
        }
        if let Some(height) = chart_config.height {
            config.height = height;
        }
        if let Some(show_legend) = chart_config.show_legend {
            config.show_legend = show_legend;
        }
        if let Some(title) = &chart_config.title {
            config.title = title.clone();
        }
    }
    
    // Set default title based on metric type and repositories
    if config.title == "Repository Star Metrics" {
        let repo_names: Vec<String> = input.repositories
            .iter()
            .map(|r| format!("{}/{}", r.owner, r.name))
            .collect();
        
        let metric_name = match metric_type {
            MetricType::Position => "Cumulative Stars",
            MetricType::Speed => "Daily Star Count",
            MetricType::Acceleration => "Star Acceleration",
        };
        
        if repo_names.len() == 1 {
            config.title = format!("{} - {}", repo_names[0], metric_name);
        } else {
            config.title = format!("Multi-Repository {} Comparison", metric_name);
        }
    }
    
    config
}
