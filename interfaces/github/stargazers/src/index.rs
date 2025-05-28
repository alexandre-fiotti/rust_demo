//! GitHub GraphQL API client for stargazer data
//! 
//! Fetches repository stars in batches of 100 using cursor-based pagination.
//! Requires GitHub token with repo read access.

use chrono::{DateTime, Utc};
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use thiserror::Error;

pub struct GitHubGraphQLResult {
    pub body: String,
    pub status: StatusCode,
}

pub async fn fetch_repo_stargazers(
    token: &str,
    owner: &str,
    name: &str,
    cursor: Option<&str>,
) -> Result<GitHubGraphQLResult, FetchRepoStargazersError> {
    let graphql_query = r#"
        query getRepoStargazers($owner: String!, $name: String!, $cursor: String) {
            repository(owner: $owner, name: $name) {
                stargazers(first: 100, after: $cursor, orderBy: {field: STARRED_AT, direction: ASC}) {
                    edges {
                        starredAt
                        node {
                            login
                            email
                        }
                    }
                    pageInfo {
                        hasNextPage
                        endCursor
                    }
                }
            }
        }
    "#;

    let payload = serde_json::json!({
        "query": graphql_query,
        "variables": {
            "owner": owner,
            "name": name,
            "cursor": cursor,
        }
    });

    let client = Client::new();

    let response = client
        .post("https://api.github.com/graphql")
        .header("Authorization", format!("Bearer {token}"))
        .header("Content-Type", "application/json")
        .header("User-Agent", "rust-client")
        .json(&payload)
        .send()
        .await
        .map_err(|source| FetchRepoStargazersError::RequestSend { source })?;

    let status = response.status();

    let body = response
        .text()
        .await
        .map_err(|source| FetchRepoStargazersError::ResponseRead { source })?;

    Ok(GitHubGraphQLResult { body, status })
}

#[derive(Debug, Error)]
pub enum FetchRepoStargazersError {
    #[error("RequestSend: {source}")]
    RequestSend {
        source: reqwest::Error,
    },
    
    #[error("ResponseRead: {source}")]
    ResponseRead {
        source: reqwest::Error,
    },
}

#[derive(Debug, Deserialize)]
pub struct GraphQLResponse {
	pub data: RepositoryData,
}

#[derive(Debug, Deserialize)]
pub struct RepositoryData {
	pub repository: Option<Repository>,
}

#[derive(Debug, Deserialize)]
pub struct Repository {
	pub stargazers: StargazerConnection,
}

#[derive(Debug, Deserialize)]
pub struct StargazerConnection {
	pub edges: Vec<StargazerEdge>,
	#[serde(rename = "pageInfo")]
	pub page_info: PageInfo,
}

#[derive(Debug, Deserialize)]
pub struct StargazerEdge {
	#[serde(rename = "starredAt")]
	pub starred_at: DateTime<Utc>,
	pub node: StargazerUser,
}

#[derive(Debug, Deserialize)]
pub struct StargazerUser {
	pub login: String,
	pub email: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PageInfo {
	#[serde(rename = "hasNextPage")]
	pub has_next_page: bool,
	#[serde(rename = "endCursor")]
	pub end_cursor: Option<String>,
}

