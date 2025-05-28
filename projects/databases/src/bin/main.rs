use std::net::SocketAddr;

use anyhow::Result;
use axum::{
	http::StatusCode, response::IntoResponse, routing::{get, post}, serve, Extension, Router
};
use utils_trace::tracing_init;
use thiserror::Error;
use tracing::{error, info};
use projects_databases::endpoints::github::repo_stars::{update::index::handler as github_repo_stars_update_handler, read_daily_data::index::handler as github_repo_stars_read_daily_data_handler, read_daily_graph::index::handler as github_repo_stars_read_daily_graph_handler};
use diesel::{r2d2::{ConnectionManager, Pool}, PgConnection};
use dotenvy::dotenv;

pub type PgPool = Pool<ConnectionManager<PgConnection>>;

#[derive(Debug, Error)]
pub enum MainError {
    #[error("TracingInit: {source}")]
    TracingInit {
        #[source]
        source: utils_trace::TracingInitError,
    },
	#[error("EnvVarSetup: {source}")]
	EnvVarSetup {
		#[source]
		source: dotenvy::Error,
	},
	#[error("DbEnvVar: {source}")]
	DbEnvVar {
		#[source]
		source: std::env::VarError,
	},
	#[error("DbPoolBuild: {source}")]
	DbPoolBuild {
		#[source]
		source: r2d2::Error,
	},
	#[error("TcpListenerBind: {source}")]
	TcpListenerBind {
		#[source]
		source: std::io::Error,
	},
	#[error("Serve: {source}")]
	Serve {
		#[source]
		source: std::io::Error,
	}
}

#[tokio::main]
async fn main() -> Result<(), MainError> {
    tracing_init("info")
        .map_err(|source| MainError::TracingInit { source })?;
	
	// Load environment variables from .env file
	dotenv().map_err(|source| MainError::EnvVarSetup { source })?;

	// Set up the database connection pool
	let db_pool = PgPool::builder()
    	.build(ConnectionManager::new(std::env::var("DATABASE_URL").map_err(|source| MainError::DbEnvVar { source })?))
    	.map_err(|source| MainError::DbPoolBuild { source })?;
 
	// Set up the router
	let app = Router::new()
		.route("/github/repo_stars/update", post(github_repo_stars_update_handler))
		.route("/github/repo_stars/read_daily_data", get(github_repo_stars_read_daily_data_handler))
		.route("/github/repo_stars/read_daily_graph", post(github_repo_stars_read_daily_graph_handler))
		.layer(Extension(db_pool.clone()));

	let addr = SocketAddr::from(([0, 0, 0, 0], 8000));
	let listener = tokio::net::TcpListener::bind(addr)
		.await
		.map_err(|source| MainError::TcpListenerBind { source })?;

	info!("Server running on addr: {}", addr);

	serve(listener, app)
		.await
		.map_err(|source| MainError::Serve { source })?;

	Ok(())
}

impl IntoResponse for MainError {
	fn into_response(self) -> axum::response::Response {
		let err = self;
        let (status, message) = (
  				StatusCode::INTERNAL_SERVER_ERROR,
  				format!("Server error: {err}"),
  			);

		(status, message).into_response()
	}
}