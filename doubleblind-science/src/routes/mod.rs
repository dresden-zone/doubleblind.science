use axum::routing::get;
use axum::Router;
use serde::{Deserialize, Deserializer};

use crate::routes::auth::{auth_login_github, auth_login_github_callback, auth_me};
use crate::state::DoubleBlindState;

mod auth;

pub(crate) fn route() -> Router<DoubleBlindState> {
    Router::new()
        .route("/auth/login/github", get(auth_login_github))
        .route("/auth/callback/github", get(auth_login_github_callback))
        .route("/auth/me", get(auth_me))
}
