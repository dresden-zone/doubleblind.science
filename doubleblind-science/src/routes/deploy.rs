use crate::service::deploy::DeploymentInformation;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use hmac::Mac;
use serde::{Deserialize, Serialize};

use tracing::{error, info};

use crate::service::token::ResponseAccessTokens;
use crate::state::DoubleBlindState;

#[derive(Serialize, Deserialize)]
pub(super) struct OwnerInformationGithub {
  id: i64,
  name: String,
  email: String,
}

#[derive(Serialize, Deserialize)]
pub(super) struct RepositoryInformationGithub {
  id: i64,
  full_name: String,
  size: i64,
  default_branch: String,
}

#[derive(Serialize, Deserialize)]
pub(super) struct GithubWebhookRequest {
  r#ref: String,
  before: String,
  after: String,
  repository: RepositoryInformationGithub,
}

pub(super) async fn github_deploy_webhook(
  State(mut state): State<DoubleBlindState>,
  data: Json<GithubWebhookRequest>,
) -> Result<StatusCode, StatusCode> {
  info!("New Deployment for {}", &data.repository.full_name);

  let repository = match state
    .project_service
    .get_repository(data.repository.id)
    .await
  {
    Ok(Some(value)) => value,
    Ok(None) => {
      info!(
        "github tried to call webhook for undeployed repo {}",
        data.repository.full_name
      );
      return Err(StatusCode::NOT_FOUND);
    }
    Err(e) => {
      error!("error while trying to query repo {e}");
      return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
  };

  if !repository.deployed {
    return Err(StatusCode::BAD_REQUEST);
  }

  let (domain, branch) = match (repository.domain, repository.branch) {
    (Some(new_domain), Some(new_branch)) => (new_domain, new_branch),
    _ => {
      error!("No Domain or Branch specified in database!");
      return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
  };

  if format!("refs/heads/{}", branch) != data.r#ref {
    return Ok(StatusCode::NO_CONTENT);
  }

  let github_app = match state
    .project_service
    .get_github_app_uuid(repository.github_app)
    .await
  {
    Ok(Some(value)) => value,
    Ok(None) => {
      info!(
        "no github installation with this name! - github-app-id: {}",
        &repository.github_app
      );
      return Err(StatusCode::NOT_FOUND);
    }
    Err(e) => {
      error!("error while trying to query github apps {e}");
      return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
  };
  let repos = match state
    .project_service
    .all_repos_for_installation_id(github_app.installation_id)
    .await
  {
    Ok(Some(value)) => value,
    Err(e) => {
      error!("cannot fetch repos for id {e}");
      return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
    Ok(None) => {
      error!("cannot fetch all repos");
      return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
  };

  let access_token: ResponseAccessTokens = state
    .token_service
    .fetch_access_tokens_repo(
      github_app.installation_id,
      repos.iter().map(|x| x.github_short_name.clone()).collect(),
    )
    .await
    .map_err(|e| {
      error!("error while trying to fetch access token {e}");
      StatusCode::INTERNAL_SERVER_ERROR
    })?;

  state
    .deployment_service
    .queue_deployment(DeploymentInformation {
      full_name: repository.github_full_name,
      token: access_token.token,
      domain,
      commit_id: data.after.clone(),
    })
    .await
    .map_err(|_e| {
      error!("queueing for deployment failed!");
      StatusCode::INTERNAL_SERVER_ERROR
    })?;

  Ok(StatusCode::OK)
}
