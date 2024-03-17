use axum::routing::{get, post};
use axum::Router;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::routes::deploy::github_deploy_webhook;
use crate::routes::setup::{
  github_app_deploy_website, github_app_repositories, github_create_installation,
  github_forward_user,
};
use crate::state::DoubleBlindState;

mod deploy;
mod setup;
mod auth;

#[derive(Serialize, Deserialize, Clone)]
pub struct GithubRepoInformation {
  pub id: i64,
  #[serde(rename = "name")]
  pub short_name: String,
  pub full_name: String,
  pub deployed: bool,
  pub domain: Option<String>,
  pub branch: Option<String>,
  pub last_update: OffsetDateTime,
}

#[derive(Deserialize, Eq, PartialEq, Hash)]
pub struct GithubRepoEdit {
  pub id: i64,
  pub name: String,
  pub full_name: String,
}

pub(crate) fn route() -> Router<DoubleBlindState> {
  Router::new()
    .route("/v1/github/hooks/deploy", post(github_deploy_webhook))
    .route("/v1/github/hooks/setup", post(github_create_installation))
    .route("/v1/github/hooks/setup", get(github_forward_user))
    .route("/v1/github/repos", get(github_app_repositories))
    .route("/v1/github/deploy", post(github_app_deploy_website))
}
