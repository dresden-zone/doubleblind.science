use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};




use hmac::{Hmac, Mac};

use serde::{Deserialize, Serialize};
use sha2::Sha256;

use tracing::{error, info};

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
  owner: OwnerInformationGithub,
}

#[derive(Serialize, Deserialize)]
pub(super) struct GithubWebhookRequest {
  before: String,
  after: String,
  repository: RepositoryInformationGithub,
}

pub(super) async fn github_deploy_webhook(
  State(mut state): State<DoubleBlindState>,
  headers: HeaderMap,
  raw_body: String,
) -> Result<StatusCode, StatusCode> {
  type HmacSha256 = Hmac<Sha256>;

  let hash = match headers.get("X-Hub-Signature-256") {
    Some(value) => value,
    None => {
      error!("github didn't send HMAC challange hash!");
      return Err(StatusCode::BAD_REQUEST);
    }
  };

  match HmacSha256::new_from_slice(state.github_hmac_secret.as_ref()) {
    Ok(mut mac) => {
      mac.update(raw_body.as_ref());
      let result: &[u8] = &mac.finalize().into_bytes();

      if result != hash.as_bytes().iter().as_slice() {
        error!("non github entity tried to call the webhook endpoint!");
        return Err(StatusCode::FORBIDDEN);
      }
    }
    Err(e) => {
      error!("cannot generate hmac with error {}", e);
      return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
  }

  let data: GithubWebhookRequest = match serde_json::from_str(&raw_body) {
    Ok(value) => value,
    Err(e) => {
      error!("cannot parse json body from request body {e}");
      return Err(StatusCode::BAD_REQUEST);
    }
  };

  let repository = match state
    .project_service
    .get_repository(data.repository.full_name.clone())
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

  let mut github_app = match state
    .project_service
    .get_github_app(repository.github_app)
    .await
  {
    Ok(Some(value)) => value,
    Ok(None) => {
      info!("no github installation with this name!");
      return Err(StatusCode::NOT_FOUND);
    }
    Err(e) => {
      error!("error while trying to query github apps {e}");
      return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
  };

  github_app = match state
    .project_service
    .refresh_tokens(github_app, &mut state.oauth_github_client)
    .await
  {
    Ok(value) => value,
    Err(e) => {
      error!("cannot refresh github tokens with error {e}");
      return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
  };

  state
    .deployment_service
    .deploy(
      &repository.github_name,
      &github_app.github_access_token,
      &data.after,
      repository.domain,
    )
    .await
    .map_err(|e| {
      error!("error while deploying the newest version {e}");
      StatusCode::INTERNAL_SERVER_ERROR
    })?;

  Ok(StatusCode::OK)
}
