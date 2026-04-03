//! Serves the embedded React dashboard when built with `--features embed-dashboard`.
//!
//! All routes that do NOT start with `/v1/`, `/health`, `/metrics`, or `/openapi`
//! are forwarded here and resolved against the embedded assets.
//! Unknown paths fall back to `index.html` so the React router works correctly.

use axum::{
    body::Body,
    http::{header, Response, StatusCode, Uri},
    response::IntoResponse,
};

// ── Embedded assets (release/packaged builds only) ────────────────────────────

#[cfg(feature = "embed-dashboard")]
mod embedded {
    use rust_embed::RustEmbed;

    // Path is relative to this crate root (crates/hsip-api/).
    // The dashboard must be built (`npm run build` inside dashboard/) before
    // compiling with --features embed-dashboard.
    #[derive(RustEmbed)]
    #[folder = "../../dashboard/dist/"]
    pub struct Assets;
}

// ── Handler ──────────────────────────────────────────────────────────────────

pub async fn serve(uri: Uri) -> impl IntoResponse {
    #[cfg(feature = "embed-dashboard")]
    {
        let raw = uri.path().trim_start_matches('/');
        let path = if raw.is_empty() { "index.html" } else { raw };

        match embedded::Assets::get(path) {
            Some(file) => {
                let mime = mime_guess::from_path(path).first_or_octet_stream();
                Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, mime.as_ref())
                    // Cache fingerprinted assets aggressively; index.html never
                    .header(
                        header::CACHE_CONTROL,
                        if path == "index.html" {
                            "no-cache"
                        } else {
                            "public, max-age=31536000, immutable"
                        },
                    )
                    .body(Body::from(file.data))
                    .unwrap_or_else(|_| not_found())
            }
            // SPA fallback — let React Router handle unknown paths
            None => {
                let index = embedded::Assets::get("index.html")
                    .expect("index.html must be present in dashboard/dist/");
                Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "text/html")
                    .header(header::CACHE_CONTROL, "no-cache")
                    .body(Body::from(index.data))
                    .unwrap_or_else(|_| not_found())
            }
        }
    }

    #[cfg(not(feature = "embed-dashboard"))]
    {
        // Dev mode — the dashboard is served by Vite on port 5173.
        // This handler is never called in practice because the route is not
        // registered when the feature is disabled, but we need it to compile.
        let _ = uri;
        not_found()
    }
}

fn not_found() -> Response<Body> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Body::from("Not found"))
        .unwrap()
}
