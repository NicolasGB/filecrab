mod config;
mod error;
mod model;
mod web;

use crate::{model::ModelManager, web::routes::routes};

pub use self::error::{Error, Result};

use axum::{
    body::Bytes,
    http::{header, HeaderValue},
    Router,
};
use std::{net::SocketAddr, time::Duration};
use tokio::signal;
use tower::ServiceBuilder;
use tower_http::{
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
    LatencyUnit, ServiceBuilderExt,
};
use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_target(true)
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let mm = ModelManager::new().await.map_err(|err| {
        eprintln!("{err}");
        Error::CouldNotInitModelManager
    })?;

    // Build our middleware stack
    let middleware = ServiceBuilder::new()
        // Add high level tracing/logging to all requests
        .layer(
            TraceLayer::new_for_http()
                .on_body_chunk(|chunk: &Bytes, latency: Duration, _: &tracing::Span| {
                    tracing::trace!(size_bytes = chunk.len(), latency = ?latency, "sending body chunk")
                })
                .make_span_with(DefaultMakeSpan::new().include_headers(true))
                .on_response(DefaultOnResponse::new().include_headers(true).latency_unit(LatencyUnit::Micros)),
        )
        // Box the response body so it implements `Default` which is required by axum
        .map_response_body(axum::body::boxed)
        // Compress responses
        .compression()
        // Set a `Content-Type` if there isn't one already.
        .insert_response_header_if_not_present(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );

    let routes = Router::new().merge(routes(mm)).layer(middleware);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));

    info!("{:12} - {addr}", "LISTENING");

    axum::Server::bind(&addr)
        .serve(routes.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+c handler")
    };

    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    tokio::select! {
        _ = ctrl_c => {}
        _ = terminate => {}
    }
}
