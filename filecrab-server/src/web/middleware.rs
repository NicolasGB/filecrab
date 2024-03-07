use axum::{
    extract::Request,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};

use crate::config::config;

pub async fn api_key_mw(
    // run the headers map extractor
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let key = headers.get("filecrab-key");
    match key {
        Some(token) => {
            if token != &config().API_KEY {
                tracing::warn!(
                    "someone tried to request the api with an invalid key {:?}",
                    token
                );
                return Err(StatusCode::UNAUTHORIZED);
            }

            //If the token matches we let through the request
            let response = next.run(request).await;
            Ok(response)
        }
        _ => {
            tracing::warn!("someone tried to request the api without a key");
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}
