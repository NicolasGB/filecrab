use axum::{
    Json, Router,
    body::Body,
    debug_handler,
    extract::{DefaultBodyLimit, Multipart, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use rand::distr::{Alphanumeric, SampleString};
use serde::{Deserialize, Serialize};
use tower_http::limit::RequestBodyLimitLayer;

use crate::{
    config::config,
    model::{
        ModelManager,
        asset::{Asset, AssetToCreate},
        text::{Text, TextToCreate},
    },
    web::{Error, Result, middleware::api_key_mw},
};

pub fn routes(mm: ModelManager) -> Router {
    Router::new()
        .route("/api/upload", post(upload_handler))
        .route("/api/paste", post(paste_handler))
        .route("/api/copy", get(copy_handler))
        .layer(DefaultBodyLimit::disable())
        .layer(RequestBodyLimitLayer::new(
            config().MAXIMUM_FILE_SIZE * 1024 * 1024, /* in mb */
        ))
        .route_layer(axum::middleware::from_fn(api_key_mw))
        // This route is specifically here after the route_layer so that the middleware is not
        // applied to it, downloading endpoint is open.
        .route("/api/download", get(download_handler))
        .with_state(mm)
}

#[derive(Serialize)]
struct CreateResponse {
    pub id: String,
}

#[debug_handler]
async fn upload_handler(
    State(mm): State<ModelManager>,
    mut multipart: Multipart,
) -> Result<Json<CreateResponse>> {
    //First we generate an id which will be used for the file and the db
    let token = Alphanumeric.sample_string(&mut rand::rng(), 16);

    // Prepare asset to create
    let mut asset_to_create = AssetToCreate {
        file_name: String::default(),
        encrypted: false,
        expire: None,
        memo_id: None,
    };

    //Parse multipart
    let mut has_file = false;
    while let Some(field) = multipart.next_field().await? {
        let name = field.name().unwrap_or_default().to_string();

        match name.as_str() {
            "file" => {
                has_file = true;
                asset_to_create.file_name =
                    field.file_name().ok_or(Error::MissingFileName)?.to_string();

                //Stream and upload the file
                mm.upload(&token, field).await?;
            }
            "encrypted" => {
                let encrypted_bytes = field.bytes().await?.to_vec();
                let encrypted_string = String::from_utf8_lossy(&encrypted_bytes).to_string();
                asset_to_create.encrypted = encrypted_string.to_lowercase().eq("true");
            }
            _ => {}
        }
    }

    //If we got a file, time to upload buddy
    let mut resp = CreateResponse { id: String::new() };

    if has_file {
        //First we store the reference
        let asset = Asset::create(mm.clone(), &token, asset_to_create).await?;

        //copy the id to the the response
        resp.id = asset.memo_id
    }

    Ok(Json(resp))
}

#[derive(Debug, Deserialize)]
struct DownloadParams {
    file: Option<String>,
}

#[debug_handler]
async fn download_handler(
    State(mm): State<ModelManager>,
    Query(params): Query<DownloadParams>,
) -> Result<impl IntoResponse> {
    // Read the asset from the database
    let asset = Asset::read_by_memo_id(mm.clone(), &params.file.unwrap_or_default()).await?;

    // Read the data from minio based of the id
    let data = mm.download(&asset.id.id.to_string()).await?;
    let response = Response::builder()
        .header("Content-Type", "application/octet-stream")
        .header("Content-Length", data.1)
        .header("filecrab-file-name", &asset.file_name)
        .body(Body::from_stream(data.0.bytes))
        .map_err(Error::Http)?;

    Ok(response)
}

#[debug_handler]
async fn paste_handler(
    State(mm): State<ModelManager>,
    Json(body): Json<TextToCreate>,
) -> Result<Response> {
    if body.content.is_empty() {
        return Ok(StatusCode::BAD_REQUEST.into_response());
    }
    let text = Text::create(mm.clone(), body).await?;

    let res = CreateResponse {
        id: text.memo_id.to_string(),
    };

    Ok(Json(res).into_response())
}

#[derive(Debug, Deserialize)]
struct CopyParams {
    memo_id: String,
}

#[derive(Debug, Serialize)]
struct CopyResponse {
    content: String,
}

#[debug_handler]
async fn copy_handler(
    State(mm): State<ModelManager>,
    Query(params): Query<CopyParams>,
) -> Result<Response> {
    if params.memo_id.is_empty() {
        return Ok(StatusCode::BAD_REQUEST.into_response());
    }

    // Read the text
    let text = Text::read(mm.clone(), params.memo_id).await?;

    let res = CopyResponse {
        content: text.content,
    };

    // Delete the text once it has been copied
    Text::delete(mm.clone(), text.id.id.to_string()).await?;

    Ok(Json(res).into_response())
}
