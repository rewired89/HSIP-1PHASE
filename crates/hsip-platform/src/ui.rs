//! Serves the console single-page app: embedded in the binary with the
//! `embed-console` feature, otherwise from `HSP_CONSOLE_DIR` (or
//! `./console/dist`) on disk. Unknown paths fall back to `index.html`
//! so client-side routes survive a refresh.

use axum::{
    http::{header, StatusCode, Uri},
    response::{IntoResponse, Response},
};

#[cfg(feature = "embed-console")]
#[derive(rust_embed::RustEmbed)]
#[folder = "../../console/dist"]
struct Assets;

fn respond(path: &str, bytes: Vec<u8>) -> Response {
    let mime = mime_guess_for(path);
    let cache = if path.starts_with("assets/") {
        "public, max-age=31536000, immutable"
    } else {
        "no-cache"
    };
    (
        [
            (header::CONTENT_TYPE, mime),
            (header::CACHE_CONTROL, cache.to_string()),
        ],
        bytes,
    )
        .into_response()
}

fn mime_guess_for(path: &str) -> String {
    #[cfg(feature = "embed-console")]
    {
        mime_guess::from_path(path)
            .first_or_octet_stream()
            .to_string()
    }
    #[cfg(not(feature = "embed-console"))]
    {
        match path.rsplit('.').next().unwrap_or("") {
            "html" => "text/html; charset=utf-8",
            "js" => "text/javascript",
            "css" => "text/css",
            "svg" => "image/svg+xml",
            "json" => "application/json",
            "png" => "image/png",
            "ico" => "image/x-icon",
            "woff2" => "font/woff2",
            _ => "application/octet-stream",
        }
        .to_string()
    }
}

pub async fn serve(uri: Uri, dir: Option<std::path::PathBuf>) -> Response {
    let raw = uri.path().trim_start_matches('/');
    if raw.starts_with("v1/") || raw == "health" {
        return (StatusCode::NOT_FOUND, "not found").into_response();
    }
    let path = if raw.is_empty() { "index.html" } else { raw };
    // Never allow path traversal out of the console directory.
    if path.contains("..") || path.contains('\\') {
        return (StatusCode::BAD_REQUEST, "bad path").into_response();
    }

    #[cfg(feature = "embed-console")]
    {
        let _ = &dir;
        if let Some(f) = Assets::get(path) {
            return respond(path, f.data.into_owned());
        }
        if let Some(f) = Assets::get("index.html") {
            return respond("index.html", f.data.into_owned());
        }
    }
    #[cfg(not(feature = "embed-console"))]
    {
        let root = dir.unwrap_or_else(|| std::path::PathBuf::from("console/dist"));
        if let Ok(b) = tokio::fs::read(root.join(path)).await {
            return respond(path, b);
        }
        if let Ok(b) = tokio::fs::read(root.join("index.html")).await {
            return respond("index.html", b);
        }
    }
    (
        StatusCode::NOT_FOUND,
        "Console not built. Run `npm install && npm run build` in console/, or build with --features embed-console.",
    )
        .into_response()
}
