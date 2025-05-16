use reqwest::{Client, StatusCode};
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
