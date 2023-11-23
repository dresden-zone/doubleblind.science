use axum::routing::{get, post};
use axum::Router;

use crate::routes::auth::{auth_login_github, auth_login_github_callback, auth_me};
use crate::routes::project::{create_project, user_repos};
use crate::state::DoubleBlindState;

mod auth;
mod project;

pub(crate) fn route() -> Router<DoubleBlindState> {
  Router::new()
    .route("/auth/me", get(auth_me))
    .route("/auth/login/github", get(auth_login_github))
    .route("/auth/callback/github", get(auth_login_github_callback))
    .route("/project/", post(create_project))
    .route("/repos/", get(user_repos))
}
