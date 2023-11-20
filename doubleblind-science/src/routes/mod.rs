mod auth;

use axum::routing::{get, post};
use axum::Router;

use std::collections::HashSet;
use std::sync::Arc;

use crate::routes::auth::{auth_login_github, auth_login_github_callback};
use crate::state::DoubleBlindState;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Deserializer};

pub(crate) fn route() -> Router<DoubleBlindState> {
    Router::new()
        .route("/auth/login/github", get(auth_login_github))
        .route("/auth/callback/github", get(auth_login_github_callback))
}
