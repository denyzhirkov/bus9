use axum::{
    http::{header, StatusCode, Uri},
    response::IntoResponse,
};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "front/dist"]
struct Assets;

pub async fn static_handler(uri: Uri) -> impl IntoResponse {
    let mut path = uri.path().trim_start_matches('/').to_string();

    if path.is_empty() {
        path = "index.html".to_string();
    }

    match Assets::get(&path) {
        Some(content) => {
            let mime = mime_guess::from_path(&path).first_or_octet_stream();
            ([(header::CONTENT_TYPE, mime.as_ref())], content.data).into_response()
        }
        None => {
            if path.contains('.') {
                return StatusCode::NOT_FOUND.into_response();
            }
            index_handler().await.into_response()
        }
    }
}

async fn index_handler() -> impl IntoResponse {
    match Assets::get("index.html") {
        Some(content) => {
            let mime = mime_guess::from_path("index.html").first_or_octet_stream();
            ([(header::CONTENT_TYPE, mime.as_ref())], content.data).into_response()
        }
        None => (StatusCode::NOT_FOUND, "index.html not found").into_response(),
    }
}
