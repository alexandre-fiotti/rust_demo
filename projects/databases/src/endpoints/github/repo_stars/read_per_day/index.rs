use axum::{
    extract::{Extension, Json},
    http::StatusCode,
    response::IntoResponse,
};

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
	#[error("RepositoryNotInDatabase: {owner}/{name}")]
	RepositoryNotInDatabase {
		owner: String,
		name: String,
	},
    #[error(transparent)]
    GetDailyStarCount{ 
		#[from] 
		source: crate::db::star::queries::GetDailyStarCountError 
	},
}

impl IntoResponse for HandlerError {
	fn into_response(self) -> axum::response::Response {
		match self {
			HandlerError::GetConnectionFromPool{ source } => (StatusCode::INTERNAL_SERVER_ERROR, source.to_string()).into_response(),
			HandlerError::GetRepositoryByName{ source } => (StatusCode::INTERNAL_SERVER_ERROR, source.to_string()).into_response(),
			HandlerError::RepositoryNotInDatabase{ owner, name } => (StatusCode::NOT_FOUND, format!("Repository {owner}/{name} not found in database")).into_response(),
			HandlerError::GetDailyStarCount{ source } => (StatusCode::INTERNAL_SERVER_ERROR, source.to_string()).into_response(),
        }
    }
}

/// JSON payload expected by the endpoint.
#[derive(Deserialize)]
pub struct RepoQuery {
	owner: String,
	name:  String,
}


/// Axum handler: POST /sync-stars
pub async fn handler(
    Extension(pool): Extension<PgPool>,
    Json(input): Json<RepoQuery>,
) -> impl IntoResponse {
 	let mut conn = match pool.get() {
    	Ok(c) => c,
    	Err(source) => return HandlerError::GetConnectionFromPool { source }.into_response(),
	};

    let repo = match get_repository_by_name(&mut conn, &input.owner, &input.name).await {
	    Ok(Some(repo)) => repo,
	    Ok(None) => {
	        return HandlerError::RepositoryNotInDatabase {
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
 
	(StatusCode::OK, Json(star_counts)).into_response()
}
