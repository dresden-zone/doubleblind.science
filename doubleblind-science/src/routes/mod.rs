use crate::routes::setup::{github_app_repositories, github_setup_webhook};
use axum::routing::{get, post};
use axum::Router;

//use crate::routes::auth::{auth_login_github, auth_login_github_callback, auth_me};
use crate::routes::deploy::github_deploy_webhook;
use crate::state::DoubleBlindState;

mod deploy;
mod setup;

pub(crate) fn route() -> Router<DoubleBlindState> {
  Router::new()
    .route("/hooks/github/deploy", get(github_deploy_webhook))
    .route("/hooks/github/setup", get(github_setup_webhook))
    .route("/repos", get(github_app_repositories))
}
