use axum::{
    extract::{Extension, Json},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use chrono::NaiveDate;
use plotters::prelude::*;
use serde::Deserialize;
use thiserror::Error;

use crate::db::{
    repository::queries::get_repository_by_name,
    star::queries::get_daily_star_count,
    PgPool,
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
    #[error("ChartGeneration: {message}")]
    ChartGeneration {
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
            HandlerError::ChartGeneration{ message } => (StatusCode::INTERNAL_SERVER_ERROR, format!("Chart generation failed: {message}")).into_response(),
        }
    }
}

/// Query parameters for the endpoint.
#[derive(Deserialize)]
pub struct RepoStarsReadDailyGraphRequestBody {
    owner: String,
    name:  String,
}

/// Axum handler: GET /github/repo_stars/read_daily_graph
pub async fn handler(
    Extension(pool): Extension<PgPool>,
    Json(input): Json<RepoStarsReadDailyGraphRequestBody>,
) -> impl IntoResponse {
    let mut conn = match pool.get() {
        Ok(c) => c,
        Err(source) => return HandlerError::GetConnectionFromPool { source }.into_response(),
    };

    let repo = match get_repository_by_name(&mut conn, &input.owner, &input.name).await {
        Ok(Some(repo)) => repo,
        Ok(None) => {
            return HandlerError::RepositoryNotFound {
                owner: input.owner.clone(),
                name: input.name.clone(),
            }
            .into_response()
        }
        Err(source) => return HandlerError::GetRepositoryByName { source }.into_response(),
    };
    
    let star_counts = match get_daily_star_count(&mut conn, repo.id) {
        Ok(data) => data,
        Err(source) => return HandlerError::GetDailyStarCount { source }.into_response(),
    };

    // Generate SVG chart
    match generate_star_chart(&star_counts, &input.owner, &input.name) {
        Ok(svg_content) => {
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "image/svg+xml")
                .header(header::CACHE_CONTROL, "public, max-age=3600")
                .body(svg_content.into())
                .unwrap()
        }
        Err(err) => HandlerError::ChartGeneration { message: err }.into_response(),
    }
}

fn generate_star_chart(
    data: &[(NaiveDate, i64)], 
    owner: &str, 
    name: &str
) -> Result<String, String> {
    if data.is_empty() {
        return Ok(generate_empty_chart(owner, name));
    }

    // Convert daily counts to cumulative counts
    let mut cumulative_data = Vec::new();
    let mut running_total = 0i64;
    
    for (date, daily_count) in data {
        running_total += daily_count;
        cumulative_data.push((*date, running_total));
    }

    let mut buffer = String::new();
    {
        let root = SVGBackend::with_string(&mut buffer, (800, 400)).into_drawing_area();
        root.fill(&WHITE).map_err(|e| format!("Failed to fill background: {}", e))?;

        let (min_date, max_date) = (
            cumulative_data.first().unwrap().0,
            cumulative_data.last().unwrap().0,
        );
        
        let max_stars = cumulative_data.iter().map(|(_, count)| *count).max().unwrap_or(0);
        let y_max = if max_stars == 0 { 10 } else { max_stars + (max_stars / 10).max(1) };

        let mut chart = ChartBuilder::on(&root)
            .caption(&format!("{}/{} - Cumulative Star Count", owner, name), ("Arial", 24).into_font())
            .margin(20)
            .x_label_area_size(50)
            .y_label_area_size(60)
            .build_cartesian_2d(min_date..max_date, 0i64..y_max)
            .map_err(|e| format!("Failed to build chart: {}", e))?;

        chart
            .configure_mesh()
            .x_desc("Date")
            .y_desc("Total Stars")
            .x_label_formatter(&|date| date.format("%m/%d").to_string())
            .y_label_formatter(&|y| format!("{}", y))
            .draw()
            .map_err(|e| format!("Failed to configure mesh: {}", e))?;

        // Draw the line chart
        chart
            .draw_series(LineSeries::new(
                cumulative_data.iter().map(|(date, count)| (*date, *count)),
                BLUE.stroke_width(2),
            ))
            .map_err(|e| format!("Failed to draw line series: {}", e))?
            .label("Total Stars")
            .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 10, y)], BLUE));

        // Draw data points
        chart
            .draw_series(PointSeries::of_element(
                cumulative_data.iter().map(|(date, count)| (*date, *count)),
                3,
                BLUE,
                &|coord, size, style| Circle::new(coord, size, style.filled()),
            ))
            .map_err(|e| format!("Failed to draw points: {}", e))?;

        chart
            .configure_series_labels()
            .background_style(&WHITE.mix(0.8))
            .border_style(&BLACK)
            .draw()
            .map_err(|e| format!("Failed to draw legend: {}", e))?;

        root.present().map_err(|e| format!("Failed to present chart: {}", e))?;
    }

    Ok(buffer)
}

fn generate_empty_chart(owner: &str, name: &str) -> String {
    format!(
        "<svg width=\"800\" height=\"400\" xmlns=\"http://www.w3.org/2000/svg\">\
            <rect width=\"100%\" height=\"100%\" fill=\"white\"/>\
            <text x=\"400\" y=\"200\" text-anchor=\"middle\" font-family=\"Arial\" font-size=\"18\" fill=\"#666666\">\
                No star data available for {}/{}\
            </text>\
        </svg>",
        owner, name
    )
}
