mod config;
mod error;
mod model;
mod web;

use crate::{model::ModelManager, web::routes::routes};

pub use self::error::{Error, Result};

use axum::Router;
use std::net::SocketAddr;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_target(true)
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let mm = ModelManager::new().await;

    let routes = Router::new().merge(routes(mm));

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));

    info!("{:12} - {addr}", "LISTENING");

    axum::Server::bind(&addr)
        .serve(routes.into_make_service())
        .await
        .unwrap();

    Ok(())
}
