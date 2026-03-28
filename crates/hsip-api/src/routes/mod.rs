pub mod agents;
pub mod audit;
pub mod consent;
pub mod credentials;
pub mod dns;
pub mod identity;
pub mod keys;
pub mod messages;
pub mod tenant;

use axum::{
    routing::{delete, get, post},
    Router,
};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        // Identity
        .route("/v1/identity",                post(identity::create_or_get))
        .route("/v1/identity",                get(identity::get))
        .route("/v1/identity/rotate",         post(identity::rotate))
        // Consent
        .route("/v1/consent",                 get(consent::list))
        .route("/v1/consent/:peer_key",       get(consent::get))
        .route("/v1/consent/grant",           post(consent::grant))
        .route("/v1/consent/revoke",          post(consent::revoke))
        // Messages
        .route("/v1/messages",                get(messages::list))
        .route("/v1/messages/sign",           post(messages::sign))
        .route("/v1/messages/verify",         post(messages::verify))
        // Credentials
        .route("/v1/credentials",             get(credentials::list))
        .route("/v1/credentials/issue",       post(credentials::issue))
        .route("/v1/credentials/verify",      post(credentials::verify))
        .route("/v1/credentials/:id/revoke",  delete(credentials::revoke))
        // Audit
        .route("/v1/audit",                   get(audit::list))
        // API Keys
        .route("/v1/keys",                    get(keys::list))
        .route("/v1/keys",                    post(keys::create))
        .route("/v1/keys/:id",                delete(keys::revoke))
        // AI Agents
        .route("/v1/agents",                  get(agents::list))
        .route("/v1/agent/capabilities",      get(agents::capabilities))
        // Tenant
        .route("/v1/tenant",                  get(tenant::info))
        .route("/v1/tenant/erase",            post(tenant::erase))
        // DNS Resolver
        .route("/v1/dns/status",              get(dns::status))
        .route("/v1/dns/enable",              post(dns::enable))
        .route("/v1/dns/disable",             post(dns::disable))
        .route("/v1/dns/log",                 get(dns::log))
}
