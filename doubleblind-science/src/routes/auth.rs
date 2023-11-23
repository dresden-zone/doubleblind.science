use axum::debug_handler;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect};
use axum::Json;
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use oauth2::reqwest::async_http_client;
use oauth2::{AuthorizationCode, CsrfToken, Scope, TokenResponse};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::sync::Arc;
use time::{Duration, OffsetDateTime};
use tracing::error;
use url::Url;
use uuid::Uuid;

use crate::auth::{Session, SessionData, SESSION_COOKIE};
use crate::state::DoubleBlindState;
use crate::structs::GithubUserInfo;

#[derive(Deserialize)]
pub(super) struct AuthCall {
  code: String,
  state: String,
}

#[derive(Serialize)]
pub(super) struct ReturnUrl {
  url: Url,
}

#[derive(Serialize)]
pub(super) struct UserInformation {
  id: Uuid,
  github_id: i64,
}

const CSRF_COOKIE: &str = "csrf_state_id";

pub(super) async fn auth_login_github(
  State(state): State<DoubleBlindState>,
  jar: CookieJar,
) -> impl IntoResponse {
  let (authorize_url, csrf_state) = state
    .oauth_github_client
    .authorize_url(CsrfToken::new_random)
    // This example is requesting access to the user's public repos and email.
    // TODO: add offline access scope
    .add_scope(Scope::new("repo".to_string()))
    .add_scope(Scope::new("user:email".to_string()))
    .add_scope(Scope::new("write:repo_hook".to_string()))
    .add_scope(Scope::new("offline".to_string()))
    .url();

  let csrf_state_id = Uuid::new_v4();

  state
    .csrf_state
    .lock()
    .await
    .insert(csrf_state_id, csrf_state);

  // Build the cookie
  let cookie = Cookie::build(CSRF_COOKIE, csrf_state_id.to_string())
    .domain("api.science.tanneberger.me")
    .same_site(SameSite::Lax)
    .path("/auth")
    .secure(true)
    .http_only(true)
    .max_age(Duration::minutes(30))
    .finish();

  (
    jar.add(cookie),
    Redirect::to(&format!("{}&access_type=offline", authorize_url)),
  )
}

pub(super) async fn auth_login_github_callback(
  State(mut state): State<DoubleBlindState>,
  Query(query): Query<AuthCall>,
  jar: CookieJar,
) -> Result<(CookieJar, Redirect), Redirect> {
  const ERROR_REDIRECT: &str = "https://science.tanneberger.me/";
  const SUCCESS_REDIRECT: &str = "https://science.tanneberger.me/projects";

  let csrf_cookie = jar.get(CSRF_COOKIE).ok_or(Redirect::to(ERROR_REDIRECT))?;

  let csrf_state_id =
    Uuid::from_str(csrf_cookie.value()).map_err(|_| Redirect::to(ERROR_REDIRECT))?;

  let csrf_state = state
    .csrf_state
    .lock()
    .await
    .remove(&csrf_state_id)
    .ok_or(Redirect::to(ERROR_REDIRECT))?;

  let code = AuthorizationCode::new(query.code.clone());
  if &query.state != csrf_state.secret() {
    return Err(Redirect::to(ERROR_REDIRECT));
  }

  let token = state
    .oauth_github_client
    .exchange_code(code)
    .request_async(async_http_client)
    .await
    .map_err(|_| Redirect::to(ERROR_REDIRECT))?;

  let access_token = token.access_token().secret().clone();

  let client = reqwest::Client::new();

  let res: GithubUserInfo = client
    .get("https://api.github.com/user")
    .header(reqwest::header::ACCEPT, "application/vnd.github+json")
    .header(
      reqwest::header::AUTHORIZATION,
      format!("Bearer {}", access_token.clone()),
    )
    .header("X-GitHub-Api-Version", "2022-11-28")
    .header(reqwest::header::USER_AGENT, "doubleblind-science")
    .send()
    .await
    .map_err(|_| Redirect::to(ERROR_REDIRECT))?
    .json()
    .await
    .map_err(|_| Redirect::to(ERROR_REDIRECT))?;

  let user = if let Ok(Some(user)) = state.user_service.get_user_by_github(res.id).await {
    // update user token
    state
      .user_service
      .update_github_access_token(
        user.id,
        &access_token,
        OffsetDateTime::now_utc() + Duration::days(15),
      )
      .await
      .map_err(|err| {
        error!("Unable to update access token: {}", err);
        Redirect::to(ERROR_REDIRECT)
      })?;

    user
  } else {
    let refresh_token = token
      .refresh_token()
      .ok_or(Redirect::to(ERROR_REDIRECT))?
      .secret()
      .to_string();

    // create new user
    state
      .user_service
      .create_github_user(
        res.id,
        refresh_token,
        OffsetDateTime::now_utc() + Duration::minutes(14),
        access_token,
        OffsetDateTime::now_utc() + Duration::days(15),
      )
      .await
      .map_err(|err| {
        error!("Unable to create github user: {}", err);
        Redirect::to(ERROR_REDIRECT)
      })?
  };

  let session_id = Uuid::new_v4();
  let session_data = SessionData { user_id: user.id };

  state
    .sessions
    .write()
    .await
    .insert(session_id, Arc::new(session_data));

  let session_cookie = Cookie::build(SESSION_COOKIE, session_id.to_string())
    .domain("api.science.tanneberger.me")
    .same_site(SameSite::Lax)
    .path("/auth")
    .secure(true)
    .http_only(true)
    .max_age(Duration::days(1))
    .finish();

  Ok((jar.add(session_cookie), Redirect::to(SUCCESS_REDIRECT)))
}

#[debug_handler]
pub(super) async fn auth_me(
  State(mut state): State<DoubleBlindState>,
  Session(session): Session,
) -> Result<Json<UserInformation>, StatusCode> {
  match state.user_service.get_user(session.user_id).await {
    Ok(Some(user_data)) => {
      if let Some(github_id) = user_data.github_user_id {
        Ok(Json(UserInformation {
          id: session.user_id,
          github_id,
        }))
      } else {
        Err(StatusCode::INTERNAL_SERVER_ERROR)
      }
    }
    Err(e) => {
      error!("while searching for user in database {:?}", e);
      Err(StatusCode::INTERNAL_SERVER_ERROR)
    }
    _ => Err(StatusCode::NOT_FOUND),
  }
}
