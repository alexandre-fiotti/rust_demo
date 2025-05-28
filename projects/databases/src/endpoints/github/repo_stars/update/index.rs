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
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;
use diesel::PgConnection;
use std::env;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::HashMap;
use reqwest::Client;

use crate::db::{
	    repository::{
	        models::NewRepository,
	        queries::{insert_repository, InsertRepositoryError},
	    },
	    star::{
	        models::NewStar,
	        queries::{insert_stars_batch, InsertStarsBatchError},
	    }, PgPool,
	};

// Job status tracking
#[derive(Debug, Clone, Serialize)]
pub struct JobStatus {
    pub id: Uuid,
    pub status: JobState,
    pub progress: JobProgress,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub enum JobState {
    Pending,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize)]
pub struct JobProgress {
    pub current_page: u32,
    pub total_stars_processed: u32,
    pub estimated_total_stars: Option<u32>,
    pub message: String,
}

// Global job tracker (in production, use Redis or database)
pub type JobTracker = Arc<Mutex<HashMap<Uuid, JobStatus>>>;

#[derive(Debug, Error)]
pub enum HandlerError {
	#[error("GetConnectionFromPool: {source}")]
	GetConnectionFromPool {
		#[from]
		source: r2d2::Error,
	},
    #[error("MissingGithubToken")]
    MissingGithubToken,
    #[error("JobSpawn: {message}")]
    JobSpawn {
        message: String,
    },
}

impl IntoResponse for HandlerError {
	fn into_response(self) -> axum::response::Response {
		match self {
            HandlerError::GetConnectionFromPool{ source } => (StatusCode::INTERNAL_SERVER_ERROR, source.to_string()).into_response(),
            HandlerError::MissingGithubToken => (StatusCode::INTERNAL_SERVER_ERROR, "GITHUB_TOKEN environment variable is not set").into_response(),
            HandlerError::JobSpawn{ message } => (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to spawn job: {}", message)).into_response(),
        }
    }
}

/// JSON payload expected by the endpoint.
#[derive(Deserialize, Clone)]
pub struct RepoQuery {
	owner: String,
	name:  String,
    /// Optional webhook URL to notify when job completes
    webhook_url: Option<String>,
}

/// Response when job is started
#[derive(Serialize)]
pub struct JobStartResponse {
    pub job_id: Uuid,
    pub message: String,
    pub status_endpoint: String,
}

/// Axum handler: POST /github/repo_stars/update
pub async fn handler(
    Extension(pool): Extension<PgPool>,
    Extension(job_tracker): Extension<JobTracker>,
    Json(input): Json<RepoQuery>,
) -> impl IntoResponse {
    let token = match env::var("GITHUB_TOKEN") {
        Ok(token) => token,
        Err(_) => return HandlerError::MissingGithubToken.into_response(),
    };

    // Create job
    let job_id = Uuid::new_v4();
    let now = Utc::now().naive_utc();
    
    let job_status = JobStatus {
        id: job_id,
        status: JobState::Pending,
        progress: JobProgress {
            current_page: 0,
            total_stars_processed: 0,
            estimated_total_stars: None,
            message: "Job queued".to_string(),
        },
        created_at: now,
        updated_at: now,
        error: None,
    };

    // Store job status
    {
        let mut tracker = job_tracker.lock().await;
        tracker.insert(job_id, job_status);
    }

    // Spawn background task
    let pool_clone = pool.clone();
    let job_tracker_clone = job_tracker.clone();
    let input_clone = input.clone();
    let token_clone = token.clone();
    
    tokio::spawn(async move {
        let result = process_repo_stars_async(
            pool_clone,
            job_tracker_clone.clone(),
            job_id,
            &token_clone,
            &input_clone,
        ).await;

        // Update final status
        let mut tracker = job_tracker_clone.lock().await;
        if let Some(job) = tracker.get_mut(&job_id) {
            match result {
                Ok(_) => {
                    job.status = JobState::Completed;
                    job.progress.message = "All stars processed successfully".to_string();
                }
                Err(e) => {
                    job.status = JobState::Failed;
                    job.error = Some(e.to_string());
                    job.progress.message = "Processing failed".to_string();
                }
            }
            job.updated_at = Utc::now().naive_utc();
            
            // Send webhook notification if URL provided
            if let Some(webhook_url) = &input_clone.webhook_url {
                let job_clone = job.clone();
                let webhook_url_clone = webhook_url.clone();
                tokio::spawn(async move {
                    send_webhook_notification(&webhook_url_clone, &job_clone).await;
                });
            }
        }
    });

    let response = JobStartResponse {
        job_id,
        message: "Star synchronization job started".to_string(),
        status_endpoint: format!("/github/repo_stars/job_status/{}", job_id),
    };

    (StatusCode::ACCEPTED, Json(response)).into_response()
}

// Job status endpoint handler
pub async fn job_status_handler(
    Extension(job_tracker): Extension<JobTracker>,
    axum::extract::Path(job_id): axum::extract::Path<Uuid>,
) -> impl IntoResponse {
    let tracker = job_tracker.lock().await;
    
    match tracker.get(&job_id) {
        Some(job) => (StatusCode::OK, Json(job.clone())).into_response(),
        None => (StatusCode::NOT_FOUND, "Job not found").into_response(),
    }
}

#[derive(Debug, Error)]
pub enum ProcessRepoStarsError {
	#[error("GetConnectionFromPool: {source}")]
	GetConnectionFromPool {
		#[from]
		source: r2d2::Error,
	},
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

async fn process_repo_stars_async(
    pool: PgPool,
    job_tracker: JobTracker,
    job_id: Uuid,
    token: &str,
    q: &RepoQuery,
) -> Result<(), ProcessRepoStarsError> {
    // Update job status to running
    {
        let mut tracker = job_tracker.lock().await;
        if let Some(job) = tracker.get_mut(&job_id) {
            job.status = JobState::Running;
            job.progress.message = "Starting to fetch repository data".to_string();
            job.updated_at = Utc::now().naive_utc();
        }
    }

    let mut conn = pool.get()?;

    // First page guarantees repo's existence and gives us initial data
    let first = fetch_chunk_of_stars_from_repo(token, &q.owner, &q.name, None).await?;

    // Update progress
    {
        let mut tracker = job_tracker.lock().await;
        if let Some(job) = tracker.get_mut(&job_id) {
            job.progress.message = "Repository found, creating database entry".to_string();
            job.updated_at = Utc::now().naive_utc();
        }
    }

    let new_repo = NewRepository {
        id: Uuid::new_v4(),
        owner: &q.owner,
        name: &q.name,
    };

    let repo = insert_repository(&mut conn, &new_repo)?;

    // Process first page
    let fetched_at = Utc::now().naive_utc();
    upsert_stars_batch(&mut conn, &repo.id, &first.stars, fetched_at)?;

    // Update progress
    {
        let mut tracker = job_tracker.lock().await;
        if let Some(job) = tracker.get_mut(&job_id) {
            job.progress.current_page = 1;
            job.progress.total_stars_processed = first.stars.len() as u32;
            job.progress.message = format!("Processed page 1, {} stars so far", first.stars.len());
            job.updated_at = Utc::now().naive_utc();
        }
    }

    let mut info = first.page_info;
    let mut cursor = info.end_cursor;
    let mut page_count = 1u32;

    while info.has_next_page {
        page_count += 1;
        
        let page = fetch_chunk_of_stars_from_repo(token, &q.owner, &q.name, cursor.as_deref()).await?;
        upsert_stars_batch(&mut conn, &repo.id, &page.stars, fetched_at)?;

        // Update progress
        {
            let mut tracker = job_tracker.lock().await;
            if let Some(job) = tracker.get_mut(&job_id) {
                job.progress.current_page = page_count;
                job.progress.total_stars_processed += page.stars.len() as u32;
                job.progress.message = format!(
                    "Processed page {}, {} stars total", 
                    page_count, 
                    job.progress.total_stars_processed
                );
                job.updated_at = Utc::now().naive_utc();
            }
        }

        info = page.page_info;
        cursor = info.end_cursor;

        // Small delay to avoid overwhelming the GitHub API
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
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
	#[error("InsertStarsBatch: {source}")]
	InsertStarsBatch{
		#[from] 
		source: InsertStarsBatchError
	},
}

#[inline]
fn upsert_stars_batch(
    conn: &mut PgConnection,
    repo_id: &Uuid,
    stars: &[StargazerEdge],
    fetched_at: NaiveDateTime,
) -> Result<(), UpsertStarsError> {
    let new_stars: Vec<NewStar> = stars
        .iter()
        .map(|star| NewStar {
            repository_id: *repo_id,
            stargazer: &star.node.login,
            starred_at: star.starred_at.naive_utc(),
            fetched_at,
        })
        .collect();

    insert_stars_batch(conn, &new_stars).map_err(|source| UpsertStarsError::InsertStarsBatch { source })?;
    Ok(())
}

/// Sends a webhook notification when a job completes
async fn send_webhook_notification(webhook_url: &str, job_status: &JobStatus) {
    let client = Client::new();
    
    let payload = serde_json::json!({
        "job_id": job_status.id,
        "status": job_status.status,
        "progress": job_status.progress,
        "completed_at": job_status.updated_at,
        "error": job_status.error
    });

    match client
        .post(webhook_url)
        .header("Content-Type", "application/json")
        .header("User-Agent", "rust-star-tracker")
        .json(&payload)
        .send()
        .await
    {
        Ok(response) => {
            if response.status().is_success() {
                tracing::info!("Webhook notification sent successfully to {}", webhook_url);
            } else {
                tracing::warn!("Webhook notification failed with status: {}", response.status());
            }
        }
        Err(e) => {
            tracing::error!("Failed to send webhook notification to {}: {}", webhook_url, e);
        }
    }
}
