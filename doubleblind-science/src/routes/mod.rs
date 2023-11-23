use crate::routes::auth::{auth_login_github, auth_login_github_callback, auth_me};
use crate::routes::project::create_project;
use crate::state::DoubleBlindState;
use axum::routing::get;
use axum::Router;

mod auth;
mod project;

pub(crate) fn route() -> Router<DoubleBlindState> {
  Router::new()
    .route("/auth/me", get(auth_me))
    .route("/auth/login/github", get(auth_login_github))
    .route("/auth/callback/github", get(auth_login_github_callback))
    .route("/project/available", get(create_project))
}
