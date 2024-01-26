use crate::routes::setup::{
  github_app_repositories, github_create_installation, github_forward_user,
};
use axum::routing::{get, post};
use axum::Router;
use jwt_simple::prelude::{Deserialize, Serialize};

//use crate::routes::auth::{auth_login_github, auth_login_github_callback, auth_me};
use crate::routes::deploy::github_deploy_webhook;
use crate::state::DoubleBlindState;

mod deploy;
mod setup;

#[derive(Serialize, Deserialize, Eq, PartialEq, Debug, Hash)]
pub(super) struct RepoInformation {
  pub id: i64,
  pub short_name: String,
  pub full_name: String,
}

pub(crate) fn route() -> Router<DoubleBlindState> {
  Router::new()
    .route("/v1/github/hooks/deploy", post(github_deploy_webhook))
    .route("/v1/github/hooks/setup", post(github_create_installation))
    .route("/v1/github/hooks/setup", get(github_forward_user))
    .route("/v1/github/repos", get(github_app_repositories))
    .route("/v1/github/deploy", post(github_app_repositories))
}
