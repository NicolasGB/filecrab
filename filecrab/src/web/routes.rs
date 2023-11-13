use axum::{
    body::{Body, Bytes},
    debug_handler,
    extract::{DefaultBodyLimit, Multipart, Query, State},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tower_http::limit::RequestBodyLimitLayer;
use tracing::info;

use crate::{
    config::config,
    model::{
        asset::{Asset, AssetToCreate},
        ModelManager,
    },
    web::{Error, Result},
};

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

#[derive(Serialize)]
struct CreateReponse {
    pub id: String,
}

#[debug_handler]
async fn upload_handler(
    State(mm): State<ModelManager>,
    mut multipart: Multipart,
) -> Result<Json<CreateReponse>> {
    let mut file_bytes: Option<Bytes> = None;

    let mut asset_to_create: AssetToCreate = AssetToCreate {
        file_name: String::default(),
        password: None,
    };

    while let Some(field) = multipart.next_field().await? {
        let name = field.name().unwrap_or_default().to_string();

        match name.as_str() {
            "file" => {
                asset_to_create.file_name = field
                    .file_name()
                    .ok_or(Error::FilenameNotFound)?
                    .to_string();

                file_bytes = Some(field.bytes().await?);
            }
            "password" => {
                let password_bytes = field.bytes().await?.to_vec();
                asset_to_create.password =
                    Some(String::from_utf8_lossy(&password_bytes).to_string());
            }
            _ => {}
        }
    }

    //If we got a file, time to upload buddy
    let mut resp = CreateReponse { id: "".to_string() };

    if let Some(file_content) = file_bytes {
        //First we store the reference
        let ass = Asset::create(mm.clone(), &mut asset_to_create)
            .await
            .map_err(Error::ModelManager)?;

        let id = ass.id.id.to_string();
        //Then we upload
        mm.upload(&id, file_content).await?;

        //copy the id to the the response
        resp.id = id;
    }

    Ok(Json(resp))
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
