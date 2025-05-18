use axum::{
    extract::{Extension, Json},
    http::StatusCode,
    response::IntoResponse,
};
use chrono::{NaiveDateTime, Utc};
use interfaces_github_stargazers::index::{
    fetch_repo_stargazers, FetchRepoStargazersError, GitHubGraphQLResult, GraphQLResponse,
    PageInfo, StargazerEdge,
};
use serde::Deserialize;
use thiserror::Error;
use uuid::Uuid;
use diesel::PgConnection;

use crate::db::{
	    repository::{
	        models::NewRepository,
	        queries::{insert_repository, InsertRepositoryError},
	    },
	    star::{
	        models::NewStar,
	        queries::{insert_star, InsertStarError},
	    }, PgPool,
	};

#[derive(Debug, Error)]
pub enum HandlerError {
	#[error("GetConnectionFromPool: {source}")]
	GetConnectionFromPool {
		#[from]
		source: r2d2::Error,
	},
    #[error(transparent)]
    SyncRepoStargazers{ 
		#[from] 
		source: SyncRepoStargazersError 
	},
}

impl IntoResponse for HandlerError {
	fn into_response(self) -> axum::response::Response {
		match self {
			HandlerError::SyncRepoStargazers{ source } => (StatusCode::INTERNAL_SERVER_ERROR, source.to_string()).into_response(),
            _ => StatusCode::NOT_FOUND.into_response(),
        }
    }
}

/// JSON payload expected by the endpoint.
#[derive(Deserialize)]
pub struct RepoQuery {
	token: String,
	owner: String,
	name:  String,
}


/// Axum handler: POST /sync-stars
pub async fn handler(
    Extension(pool): Extension<PgPool>,
    Json(input): Json<RepoQuery>,
) -> impl IntoResponse {
 	let mut conn = pool.get()
		.map_err(|source| { 
			HandlerError::GetConnectionFromPool{ source }
		})?;

    sync_repo_stargazers(&mut conn, &input).await.map_err(|source| { HandlerError::SyncRepoStargazers{ source } })
}

#[derive(Debug, Error)]
pub enum SyncRepoStargazersError {
	#[error("FetchChunkOfStarsFromRepo: {source}")]
	FetchChunkOfStarsFromRepo{
		#[from] 
		source: FetchChunkOfStarsFromRepoError
	},
	#[error("InsertRepository: {source}")]
	InsertRepository{
		#[from] 
		source: InsertRepositoryError
	},
	#[error("UpsertStars: {source}")]
	UpsertStars {
		#[from] 
		source: UpsertStarsError
	},
}

pub async fn sync_repo_stargazers(conn: &mut PgConnection, q: &RepoQuery) -> Result<(), SyncRepoStargazersError> {
    // 1. First page guarantees repo’s existence.
    let first = fetch_chunk_of_stars_from_repo(&q.token, &q.owner, &q.name, None)
		.await
		.map_err(|source| SyncRepoStargazersError::FetchChunkOfStarsFromRepo{ source })?;


	let new_repo = NewRepository {
        id: Uuid::new_v4(),
        owner: &q.owner,
        name:  &q.name,
    };

    let repo = insert_repository(conn, &new_repo)
		.map_err(|source| SyncRepoStargazersError::InsertRepository{ source })?;

    // 3. Persist every page of stars.
    let fetched_at = Utc::now().naive_utc();
    upsert_stars(conn, &repo.id, &first.stars, fetched_at).map_err(|source| SyncRepoStargazersError::UpsertStars{ source })?;

    let mut info = first.page_info;
    let mut cursor = info.end_cursor;

    while info.has_next_page {
        let page = fetch_chunk_of_stars_from_repo(&q.token, &q.owner, &q.name, cursor.as_deref()).await?;
        upsert_stars(conn, &repo.id, &page.stars, fetched_at).map_err(|source| SyncRepoStargazersError::UpsertStars{ source })?;

        info = page.page_info;
        cursor = info.end_cursor;
    }
    Ok(())
}

struct Page {
    stars:     Vec<StargazerEdge>,
    page_info: PageInfo,
}

#[derive(Debug, Error)]
pub enum FetchChunkOfStarsFromRepoError {
	#[error("FetchRepoStargazers: {source}")]
	FetchRepoStargazers{
		#[from] 
		source: FetchRepoStargazersError
	},
	#[error("ResponseBodyDeserialization: {source}")]
	ResponseBodyDeserialization{
		#[from] 
		source: serde_json::Error
	},
	#[error("RepositoryNotFound: {owner}/{name}")]
	RepositoryNotFound {
		owner: String,
		name:  String,
	},
}

async fn fetch_chunk_of_stars_from_repo(
    token: &str,
    owner: &str,
    name:  &str,
    cursor: Option<&str>,
) -> Result<Page, FetchChunkOfStarsFromRepoError> {
    let GitHubGraphQLResult { body, .. } =
        fetch_repo_stargazers(token, owner, name, cursor).await.map_err(|source| FetchChunkOfStarsFromRepoError::FetchRepoStargazers{ source })?;

    let parsed: GraphQLResponse = serde_json::from_str(&body).map_err(|source| FetchChunkOfStarsFromRepoError::ResponseBodyDeserialization{ source })?;
    let repo = parsed
        .data
        .repository
        .ok_or_else(|| FetchChunkOfStarsFromRepoError::RepositoryNotFound {
            owner: owner.into(),
            name:  name.into(),
        })?;

    Ok(Page {
        stars: repo.stargazers.edges,
        page_info: repo.stargazers.page_info,
    })
}

#[derive(Debug, Error)]
pub enum UpsertStarsError {
	#[error("InsertStar: {source}")]
	InsertStar{
		#[from] 
		source: InsertStarError
	},
}

#[inline]
fn upsert_stars(
    conn: &mut PgConnection,
    repo_id: &Uuid,
    stars: &[StargazerEdge],
    fetched_at: NaiveDateTime,
) -> Result<(), UpsertStarsError> {
    for star in stars {
        let new_star = NewStar {
            repository_id: *repo_id,
            stargazer:     &star.node.login,
            starred_at:    star.starred_at.naive_utc(),
            fetched_at,
        };

        insert_star(conn, &new_star).map_err(|source|UpsertStarsError::InsertStar { source })?;
    }

    Ok(())
}
