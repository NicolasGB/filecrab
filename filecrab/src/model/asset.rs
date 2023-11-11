use serde::Serialize;

#[derive(Clone, Serialize)]
pub struct Asset {
    id: String,
    password: Option<String>,
    path: String,
}

impl Asset {}
