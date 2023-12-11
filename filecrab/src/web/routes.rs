use axum::{
    body::StreamBody,
    debug_handler,
    extract::{DefaultBodyLimit, Multipart, Query, State},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use rand::distributions::{Alphanumeric, DistString};
use serde::{Deserialize, Serialize};
use tower_http::limit::RequestBodyLimitLayer;

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
            config().MAXIMUM_FILE_SIZE * 1024 * 1024, /* in mb */
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
    //First we generate an id which will be used for the file and the db
    let token = Alphanumeric.sample_string(&mut rand::thread_rng(), 16);

    // Prepare asset to create
    let mut asset_to_create = AssetToCreate {
        file_name: String::default(),
        password: None,
        expire: None,
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
            "password" => {
                let password_bytes = field.bytes().await?.to_vec();
                asset_to_create.password =
                    Some(String::from_utf8_lossy(&password_bytes).to_string());
            }
            "expire" => {
                let expire_bytes = field.bytes().await?.to_vec();
                let expire_string = String::from_utf8_lossy(&expire_bytes).to_string();
                asset_to_create.expire = Some(
                    expire_string
                        .try_into()
                        .map_err(|_| Error::InvalidExpireTime)?,
                );
            }
            _ => {}
        }
    }

    //If we got a file, time to upload buddy
    let mut resp = CreateReponse { id: "".to_string() };

    if has_file {
        //First we store the reference
        let _ = Asset::create(mm.clone(), &token, &mut asset_to_create)
            .await
            .map_err(Error::ModelManager)?;

        //copy the id to the the response
        resp.id = token.to_string();
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
        .body(StreamBody::new(data.bytes))
        .map_err(Error::Http)?;

    Ok(response)
}
