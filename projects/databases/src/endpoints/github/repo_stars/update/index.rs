use axum::{
	extract::Json,
	http::{Response, StatusCode},
	response::IntoResponse,
	body::Body,
};
use interfaces_github_stargazers::index::{
	fetch_repo_stargazers, FetchRepoStargazersError, GitHubGraphQLResult,
};
use serde::Deserialize;
use thiserror::Error;
use tracing::info;

#[derive(Deserialize)]
pub struct RepoQuery {
	token: String,
	owner: String,
	name: String,
	cursor: Option<String>,
}

#[derive(Debug, Error)]
pub enum HandlerError {
	#[error("Request to GitHub failed")]
	RequestFail {
		source: FetchRepoStargazersError,
	},
}

pub async fn handler(Json(input): Json<RepoQuery>) -> impl IntoResponse {
	let GitHubGraphQLResult { body, status } = fetch_repo_stargazers(
		&input.token,
		&input.owner,
		&input.name,
		input.cursor.as_deref(),
	)
	.await
	.map_err(|source| {
		let err = HandlerError::RequestFail { source };
		(StatusCode::BAD_GATEWAY, err.to_string())
	})?;

	Response::builder()
		.status(status)
		.header("Content-Type", "application/json")
		.body(Body::from(body))
		.map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))
}
