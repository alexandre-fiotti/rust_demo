use axum::{
	extract::Json,
	http::{Response, StatusCode},
	response::IntoResponse,
	body::Body,
};
use interfaces_github_stargazers::index::{
	fetch_repo_stargazers, FetchRepoStargazersError, GitHubGraphQLResult, GraphQLResponse};
use serde::Deserialize;
use thiserror::Error;
use chrono::NaiveDate;
use std::collections::BTreeMap;

#[derive(Deserialize)]
pub struct RepoQuery {
	token: String,
	owner: String,
	name: String,
}

#[derive(Debug, Error)]
pub enum HandlerError {
	#[error("Request to GitHub failed")]
	RequestFail {
		source: FetchAndAggregateStargazersPerDayError,
	},
}

pub async fn handler(Json(input): Json<RepoQuery>) -> impl IntoResponse {
	let stars_per_day = fetch_and_aggregate_stargazers_per_day(
		&input.token,
		&input.owner,
		&input.name,
	)
	.await
	.map_err(|source| {
		let err = HandlerError::RequestFail { source };
		(StatusCode::BAD_GATEWAY, err.to_string())
	})?;


	let json = serde_json::to_string(&stars_per_day)
		.map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

	Response::builder()
		.status(StatusCode::OK)
		.header("Content-Type", "application/json")
		.body(Body::from(json))
		.map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))
}

#[derive(Debug, Error)]
pub enum FetchAndAggregateStargazersPerDayError {
	#[error("FetchRepoStargazers: {source}")]
	FetchRepoStargazers {
		#[from]
		source: FetchRepoStargazersError,
	},

	#[error("DeserializeResponseBody: {source}")]
	DeserializeResponseBody {
		#[from]
		source: serde_json::Error,
	},

	#[error("Missing or malformed repository field in GraphQL response")]
	RepositoryFieldMissing,

	#[error("Missing or malformed pageInfo in GraphQL response")]
	PageInfoInvalid,
}

pub type StargazersAggregation = BTreeMap<NaiveDate, usize>;

pub async fn fetch_and_aggregate_stargazers_per_day(
	token: &str,
	owner: &str,
	name: &str,
) -> Result<StargazersAggregation, FetchAndAggregateStargazersPerDayError> {
	let mut aggregation = BTreeMap::new();
	let mut cursor = None;

	loop {
		let GitHubGraphQLResult { body, .. } =
			fetch_repo_stargazers(token, owner, name, cursor.as_deref()).await?;

		let parsed: GraphQLResponse = serde_json::from_str(&body)?;
		let repository = parsed
			.data
			.repository
			.ok_or(FetchAndAggregateStargazersPerDayError::RepositoryFieldMissing)?;

		for edge in repository.stargazers.edges {
			let day = edge.starred_at.date_naive();
			*aggregation.entry(day).or_insert(0) += 1;
		}

		let page_info = repository.stargazers.page_info;
		if !page_info.has_next_page {
			break;
		}

		cursor = page_info.end_cursor;
	}

	Ok(aggregation)
}


