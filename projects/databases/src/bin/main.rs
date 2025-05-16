use std::net::SocketAddr;

use anyhow::Result;
use axum::{
	http::StatusCode,
	response::IntoResponse,
	routing::{post},
	serve, Router,
};
use utils_trace::tracing_init;
use thiserror::Error;
use tracing::{error, info};
use projects_databases::endpoints::github::repo_stars::update::index::handler as github_repo_stars_update_handler;

#[derive(Debug, Error)]
pub enum MainError {
    #[error("TracingInit: {source}")]
    TracingInit {
        #[source]
        source: utils_trace::TracingInitError,
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
 
	// Set up the router
	let app = Router::new()
		.route("/github/repo_stars/update", post(github_repo_stars_update_handler));

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