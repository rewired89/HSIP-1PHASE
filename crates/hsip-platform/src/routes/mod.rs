use axum::{
    extract::DefaultBodyLimit,
    routing::{delete, get, patch, post},
    Router,
};

use crate::state::AppState;

pub mod account;
pub mod evidence;
pub mod keys;
pub mod operator;
pub mod orgs;
pub mod proofs;
pub mod public;
pub mod streams;
pub mod team;

pub fn api(state: AppState) -> Router {
    Router::new()
        // public
        .route("/health", get(public::health))
        .route("/v1/meta", get(public::meta))
        .route(
            "/v1/verify",
            post(public::verify).layer(DefaultBodyLimit::max(32 * 1024 * 1024)),
        )
        .route("/v1/verify/match", post(public::match_content))
        .route("/v1/public/shares/:token", get(public::view_share))
        .route("/v1/public/orgs/:slug", get(public::org_key))
        .route("/v1/leads", post(public::lead))
        // accounts
        .route("/v1/auth/signup", post(account::signup))
        .route("/v1/auth/login", post(account::login))
        .route("/v1/auth/logout", post(account::logout))
        .route("/v1/auth/me", get(account::me))
        .route("/v1/auth/invite/:token", get(account::invite_preview))
        .route("/v1/auth/accept-invite", post(account::accept_invite))
        .route("/v1/orgs", post(account::new_org))
        // machine API (API keys)
        .route("/v1/ingest", post(proofs::ingest))
        .route("/v1/ingest/prepare", post(proofs::ingest_prepare))
        .route("/v1/ingest/commit", post(proofs::ingest_commit))
        .route("/v1/ingest/batch", post(proofs::ingest_batch))
        .route("/v1/proofs/:id", get(proofs::api_get))
        // organization console
        .route("/v1/orgs/:org", get(orgs::get).patch(orgs::rename))
        .route("/v1/orgs/:org/key-custody", post(orgs::set_custody))
        .route("/v1/orgs/:org/overview", get(orgs::overview))
        .route("/v1/orgs/:org/usage", get(orgs::usage))
        .route("/v1/orgs/:org/plan-request", post(orgs::request_plan))
        .route("/v1/orgs/:org/service-request", post(orgs::request_service))
        .route(
            "/v1/orgs/:org/streams",
            get(streams::list).post(streams::create),
        )
        .route("/v1/orgs/:org/streams/:sid", patch(streams::update))
        .route(
            "/v1/orgs/:org/streams/:sid/proofs",
            post(proofs::console_record),
        )
        .route(
            "/v1/orgs/:org/streams/:sid/integrity",
            get(proofs::integrity),
        )
        .route("/v1/orgs/:org/proofs", get(proofs::list))
        .route("/v1/orgs/:org/proofs/:pid", get(proofs::get_one))
        .route(
            "/v1/orgs/:org/proofs/:pid/shares",
            post(proofs::create_share),
        )
        .route("/v1/orgs/:org/shares", get(proofs::list_shares))
        .route("/v1/orgs/:org/shares/:id", delete(proofs::revoke_share))
        .route("/v1/orgs/:org/api-keys", get(keys::list).post(keys::create))
        .route("/v1/orgs/:org/api-keys/:id", delete(keys::revoke))
        .route("/v1/orgs/:org/members", get(team::list))
        .route(
            "/v1/orgs/:org/members/:uid",
            patch(team::set_role).delete(team::remove),
        )
        .route("/v1/orgs/:org/invites", post(team::invite))
        .route("/v1/orgs/:org/invites/:id", delete(team::revoke_invite))
        .route(
            "/v1/orgs/:org/evidence",
            get(evidence::list).post(evidence::create),
        )
        .route("/v1/orgs/:org/evidence/:id/pack", get(evidence::pack))
        .route(
            "/v1/orgs/:org/evidence/:id/report",
            get(evidence::report_html),
        )
        // operator
        .route("/v1/operator/overview", get(operator::overview))
        .route("/v1/operator/leads", get(operator::leads))
        .route("/v1/operator/orgs/:id/plan", post(operator::set_plan))
        .route("/v1/operator/orgs/:id/status", post(operator::set_status))
        .with_state(state)
}
