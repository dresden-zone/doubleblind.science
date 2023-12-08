use std::str::FromStr;
use std::sync::Arc;

use axum::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum_extra::extract::CookieJar;
use reqwest::StatusCode;
use uuid::Uuid;

use crate::state::DoubleBlindState;

pub(crate) const SESSION_COOKIE: &str = "session_id";

pub(crate) struct SessionData {
  pub(crate) user_id: Uuid,
}

pub(crate) struct Session(pub Arc<SessionData>);

#[async_trait]
impl FromRequestParts<DoubleBlindState> for Session {
  type Rejection = StatusCode;

  async fn from_request_parts(
    parts: &mut Parts,
    state: &DoubleBlindState,
  ) -> Result<Self, Self::Rejection> {
    let jar = CookieJar::from_headers(&parts.headers);
    let cookie = jar.get(SESSION_COOKIE).ok_or(StatusCode::UNAUTHORIZED)?;
    let session_id = Uuid::from_str(cookie.value()).map_err(|_| StatusCode::UNAUTHORIZED)?;

    let data = state
      .sessions
      .read()
      .await
      .get(&session_id)
      .ok_or(StatusCode::UNAUTHORIZED)?
      .clone();

    Ok(Self(data))
  }
}
