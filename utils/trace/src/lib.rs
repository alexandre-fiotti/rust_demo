use tracing_subscriber::prelude::*;
use anyhow::Result;
use thiserror::Error;
use tracing_subscriber::{fmt, EnvFilter};


pub fn tracing_init(level: &str) -> Result<(), TracingInitError> {
    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(level))
        .map_err(|source| TracingInitError::InvalidFilter { source })?;

let subscriber = tracing_subscriber::registry()
    .with(filter)
    .with(fmt::layer().compact());

tracing::subscriber::set_global_default(subscriber)
    .map_err(|source| TracingInitError::SubscriberSetGlobalDefault { source })?;


    Ok(())
}

#[derive(Debug, Error)]
pub enum TracingInitError {
    #[error("InvalidFilter")]
    InvalidFilter {
        #[from]
        source: tracing_subscriber::filter::ParseError,
    },

    #[error("SubscriberSetGlobalDefault")]
    SubscriberSetGlobalDefault {
        #[from]
        source: tracing::subscriber::SetGlobalDefaultError,
    },
}

