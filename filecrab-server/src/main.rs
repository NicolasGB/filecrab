mod config;
mod error;
mod model;
mod web;

use crate::{
    config::config,
    model::{asset::Asset, text::Text, ModelManager},
    web::routes::routes,
};

pub use self::error::{Error, Result};

use axum::{
    body::Bytes,
    http::{header, HeaderName, HeaderValue},
    Router,
};
use clokwerk::{AsyncScheduler, TimeUnits};
use std::{iter::once, time::Duration};
use tokio::{net::TcpListener, signal};
use tower::ServiceBuilder;
use tower_http::{
    compression::CompressionLayer,
    map_response_body::MapResponseBodyLayer,
    sensitive_headers::SetSensitiveHeadersLayer,
    set_header::SetResponseHeaderLayer,
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
    LatencyUnit,
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

    let filecrab_header = HeaderName::from_static("filecrab-key");

    // Build our middleware stack
    let middleware = ServiceBuilder::new()
        .layer(SetSensitiveHeadersLayer::new(once(filecrab_header)))
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
            .layer(MapResponseBodyLayer::new(axum::body::Body::new))
            // Compress responses
            .layer(CompressionLayer::new())
            // Set a `Content-Type` if there isn't one already.
            .layer(SetResponseHeaderLayer::if_not_present(
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/json"),
            ));

    // Setup cleaning with a schedule
    let mut scheduler = AsyncScheduler::with_tz(chrono::Utc);
    // Clone because borrowchecker :)
    let mmc = mm.clone();
    scheduler
        .every(config().CLEANUP_INTERVAL.seconds())
        .run(move || {
            let mmc = mmc.clone();
            async move { clean_database(mmc).await }
        });
    // Spawn task that will run and clean
    tokio::spawn(async move {
        loop {
            // Run pending jobs
            scheduler.run_pending().await;
            // Sleep for a second
            tokio::time::sleep(Duration::from_millis(1000)).await;
        }
    });

    let routes = Router::new().merge(routes(mm.clone())).layer(middleware);

    let listener = TcpListener::bind("0.0.0.0:8080")
        .await
        .map_err(|_| Error::CouldNotInitTcpListener("Could not start the listener"))?;

    info!("{:12} - {:?}", "LISTENING", listener.local_addr());

    axum::serve(listener, routes.into_make_service())
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

async fn clean_database(mm: ModelManager) {
    info!("Cleaning databases");
    let res = Asset::clean_assets(mm.clone()).await.unwrap();
    // Delete assets from the minio
    mm.delete_files(res).await.unwrap();

    // Delete text
    Text::clean_text(&mm).await.unwrap();
}
