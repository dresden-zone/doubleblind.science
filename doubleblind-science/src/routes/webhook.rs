use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum_extra::extract::cookie::CookieJar;

use hmac::{Hmac, Mac};

use serde::{Deserialize, Serialize};
use sha2::Sha256;

use time::OffsetDateTime;
use tracing::error;

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

pub(super) async fn github_webhook(
  State(mut state): State<DoubleBlindState>,
  headers: HeaderMap,
  _jar: CookieJar,
  raw_body: String,
) -> impl IntoResponse {
  type HmacSha256 = Hmac<Sha256>;

  let hash = match headers.get("X-Hub-Signature-256") {
    Some(value) => value,
    None => {
      error!("github didn't send HMAC challange hash!");
      return StatusCode::BAD_REQUEST;
    }
  };

  match HmacSha256::new_from_slice(b"my secret and secure key") {
    Ok(mut mac) => {
      mac.update(raw_body.as_ref());
      let result: &[u8] = &mac.finalize().into_bytes();

      if result != hash.as_bytes().iter().as_slice() {
        error!("non github entity tried to call the webhook endpoint!");
        return StatusCode::FORBIDDEN;
      }
    }
    Err(e) => {
      error!("cannot generate hmac with error {}", e);
      return StatusCode::INTERNAL_SERVER_ERROR;
    }
  }
  let data: GithubWebhookRequest = match serde_json::from_str(&raw_body) {
    Ok(value) => value,
    Err(_e) => {
      return StatusCode::BAD_REQUEST;
    }
  };

  let project = match state
    .project_service
    .get_projects_by_github_id(data.repository.id)
    .await
  {
    Ok(Some(value)) => value,
    Ok(None) => {
      error!("request didnt find project with id ");
      return StatusCode::BAD_REQUEST;
    }
    Err(e) => {
      error!(
        "while trying to find project with that github id {} error {}",
        data.repository.id, e
      );
      return StatusCode::BAD_REQUEST;
    }
  };

  let user_info = match state.user_service.get_user(project.owner).await {
    Ok(Some(value)) => value,
    Ok(None) => {
      error!("cannot find user to that project!");
      return StatusCode::BAD_REQUEST;
    }
    Err(e) => {
      error!("query associated user failed with {e}");
      return StatusCode::INTERNAL_SERVER_ERROR;
    }
  };

  if let (
    Some(mut access_token),
    Some(access_token_expr),
    Some(_refresh_token),
    Some(_refresh_token_expr),
  ) = (
    user_info.github_access_token,
    user_info.github_access_token_expire,
    user_info.github_refresh_token,
    user_info.github_refresh_token_expire,
  ) {
    if access_token_expr > OffsetDateTime::now_utc() {
      match state
        .user_service
        .fresh_access_token(&mut state.oauth_github_client, user_info.id)
        .await
      {
        Some(new_token) => {
          access_token = new_token;
        }
        None => {
          return StatusCode::INTERNAL_SERVER_ERROR;
        }
      };
    }

    return match state
      .deployment_service
      .deploy(
        &data.repository.full_name,
        &access_token,
        &data.after,
        &project.domain,
      )
      .await
    {
      Err(e) => {
        error!("deployment failed with {e}");
        StatusCode::INTERNAL_SERVER_ERROR
      }
      _ => StatusCode::OK,
    };
  }

  StatusCode::INTERNAL_SERVER_ERROR
}
