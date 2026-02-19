use axum::{
    body::Body,
    http::{header, StatusCode, Uri},
    response::{IntoResponse, Response},
    Router,
};
use rust_embed::RustEmbed;

use crate::AppState;

/// Embeds the compiled Vue frontend from `frontend/dist/` at compile time.
/// For production builds, replace the folder path with `../../frontend/dist/`
/// after running `npm run build` in the `frontend/` directory.
#[derive(RustEmbed)]
#[folder = "../../frontend/dist-placeholder/"]
struct FrontendAssets;

pub fn static_router() -> Router<AppState> {
    Router::new().fallback(serve_static)
}

async fn serve_static(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };

    match FrontendAssets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path)
                .first_or_octet_stream()
                .to_string();
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, mime)
                .body(Body::from(content.data.into_owned()))
                .unwrap()
        }
        None => {
            // SPA fallback: serve index.html for all unmatched paths
            match FrontendAssets::get("index.html") {
                Some(index) => Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
                    .body(Body::from(index.data.into_owned()))
                    .unwrap(),
                None => (StatusCode::NOT_FOUND, "Frontend not built").into_response(),
            }
        }
    }
}
