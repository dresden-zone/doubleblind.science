use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum_extra::extract::cookie::CookieJar;

use hmac::{Hmac, Mac};

use serde::{Deserialize, Serialize};
use sha2::Sha256;


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

pub(super) async fn github_deploy_webhook(
  State(_state): State<DoubleBlindState>,
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

  StatusCode::INTERNAL_SERVER_ERROR
}
