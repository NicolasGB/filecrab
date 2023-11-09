use axum::{
    body::Body,
    debug_handler,
    extract::{DefaultBodyLimit, Multipart, Query, State},
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use serde::Deserialize;
use tower_http::limit::RequestBodyLimitLayer;
use tracing::info;

use crate::{config::config, model::ModelManager, web::Result};

pub fn routes(mm: ModelManager) -> Router {
    Router::new()
        .route("/api/upload", post(upload_handler))
        .route("/api/download", get(download_handler))
        .layer(DefaultBodyLimit::disable())
        .layer(RequestBodyLimitLayer::new(
            config().MAXIMUM_FILE_SIZE * 1024 * 1024, /* 250mb */
        ))
        .with_state(mm)
}

#[debug_handler]
async fn upload_handler(State(mm): State<ModelManager>, mut multipart: Multipart) -> Result<()> {
    while let Some(field) = multipart.next_field().await? {
        let name = field.name().unwrap_or_default().to_string();

        let file = field.bytes().await.unwrap();

        mm.upload(&name, file).await?
    }

    Ok(())
}

#[derive(Debug, Deserialize)]
struct DownloadParams {
    file: Option<String>,
}

async fn download_handler(
    State(mm): State<ModelManager>,
    Query(params): Query<DownloadParams>,
) -> Result<impl IntoResponse> {
    let data = mm.download(&params.file.unwrap_or_default()).await?;
    let response = Response::builder()
        .header("Content-Type", "application/octet-stream")
        .body(Body::from(data))
        .unwrap();

    Ok(response)
}
