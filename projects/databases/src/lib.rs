//! GitHub repository star tracking service
//! 
//! - REST API endpoints in `endpoints/`
//! - PostgreSQL models and queries in `db/`
//! - Requires GITHUB_TOKEN env var for API access

pub mod endpoints;
pub mod db;