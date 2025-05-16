use axum::{body::Body, http::StatusCode, response::Response};
use hyper::header::CONTENT_TYPE;
use tracing::info;

pub async fn handler() -> Response<Body> {
	Response::builder()
		.status(StatusCode::OK)
		.header(CONTENT_TYPE, "text/plain; charset=utf-8")
		.body(Body::from("OK"))
		.unwrap()
}