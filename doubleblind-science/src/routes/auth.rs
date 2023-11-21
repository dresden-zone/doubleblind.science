use std::str::FromStr;
use std::sync::Arc;

use axum::extract::{Query, State};
use axum::response::{IntoResponse, Redirect};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use oauth2::reqwest::async_http_client;
use oauth2::{AuthorizationCode, CsrfToken, Scope, TokenResponse};
use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};
use tracing::{error, info};
use url::Url;
use uuid::Uuid;

// Alternatively, this can be `oauth2::curl::http_client` or a custom client.
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

  (jar.add(cookie), Redirect::to(authorize_url.as_ref()))
}

pub(super) async fn auth_login_github_callback(
  State(mut state): State<DoubleBlindState>,
  Query(query): Query<AuthCall>,
  jar: CookieJar,
) -> Result<(CookieJar, Redirect), Redirect> {
  const ERROR_REDIRECT: &str = "https://science.tanneberger.me/";
  const SUCCESS_REDIRECT: &str = "https://science.tanneberger.me/projects";

  return if let Some(csrf_cookie) = jar.get(CSRF_COOKIE) {
    let csrf_state_id = match Uuid::from_str(csrf_cookie.value()) {
      Ok(value) => value,
      Err(e) => {
        return Err(Redirect::to(ERROR_REDIRECT));
      }
    };

    return if let Some(csrf_state) = state.csrf_state.lock().await.remove(&csrf_state_id) {
      let code = AuthorizationCode::new(query.code.clone());
      if &query.state == csrf_state.secret() {
        match state
          .oauth_github_client
          .exchange_code(code)
          .request_async(async_http_client)
          .await
        {
          Ok(token) => {
            let access_token = token.access_token().secret().clone();

            let client = reqwest::Client::new();

            let res: GithubUserInfo = match client
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
            {
              Ok(value) => {
                print!("{:?}", &value);
                match value.json::<GithubUserInfo>().await {
                  Ok(parsed_json) => parsed_json,
                  Err(e) => {
                    error!("cannot parse request body from github {:?}", e);
                    return Err(Redirect::to(ERROR_REDIRECT));
                  }
                }
              }
              Err(e) => {
                error!("error while fetching user id from github {:?}", e);
                return Err(Redirect::to(ERROR_REDIRECT));
              }
            };

            let user = if let Ok(Some(user)) = state.user_service.get_user_by_github(res.id).await {
              // update user token
              if let Err(e) = state
                .user_service
                .update_github_access_token(
                  user.id,
                  access_token,
                  OffsetDateTime::now_utc() + Duration::days(15),
                )
                .await
              {
                error!("error while trying to update github refresh token {:?}", e);
                return Err(Redirect::to(ERROR_REDIRECT));
              }
              user
            } else {
              let refresh_token = match token.refresh_token() {
                Some(unpacked_token) => unpacked_token.secret().clone(),
                None => {
                  error!("didn't get access token from github!");
                  return Err(Redirect::to(ERROR_REDIRECT));
                }
              };

              // create new user
              match state
                .user_service
                .create_github_user(
                  res.id,
                  refresh_token,
                  OffsetDateTime::now_utc() + Duration::minutes(14),
                  access_token,
                  OffsetDateTime::now_utc() + Duration::days(15),
                )
                .await
              {
                Ok(x) => x,
                Err(e) => {
                  error!("error while trying to create user {:?}", e);
                  return Err(Redirect::to(ERROR_REDIRECT));
                }
              }
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

            info!("Succesfull authenticated: {}", user.id);
            Ok((jar.add(session_cookie), Redirect::to(SUCCESS_REDIRECT)))
          }
          Err(e) => {
            Err(Redirect::to(ERROR_REDIRECT))
          }
        }
      } else {
        Err(Redirect::to(ERROR_REDIRECT))
      }
    } else {
      Err(Redirect::to(ERROR_REDIRECT))
    };
  } else {
    Err(Redirect::to(ERROR_REDIRECT))
  };
}

pub(super) async fn auth_me(
  //State(mut state): State<DoubleBlindState>,
  Session(session): Session,
) -> String {
  session.user_id.to_string()
}
