use axum::{
  extract::{Json, Query, State},
  http::StatusCode,
  response::Redirect,
};
use axum_extra::extract::{
  cookie::{Cookie, SameSite},
  CookieJar,
};
use oauth2::{reqwest::async_http_client, AuthorizationCode, TokenResponse};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use time::{Duration, OffsetDateTime};
use tracing::{error, info};
use uuid::Uuid;

use crate::auth::{Session, SessionData, SESSION_COOKIE};
use crate::state::DoubleBlindState;

#[derive(Serialize, Deserialize)]
pub(super) struct RepoInformation {
  id: i64,
  name: String,
  full_name: String,
}

#[derive(Serialize)]
pub(super) struct WebHookInformation {
  url: String,
  content_type: String,
  insecure_ssl: String,
  //token: String,
}

#[derive(Serialize)]
pub(super) struct WebhookRegistrationRequest {
  name: String,
  active: bool,
  events: Vec<String>,
  config: WebHookInformation,
}

#[derive(Serialize)]
pub(super) struct GithubDispatchEvent {
  event_type: String,
  //client_payload: serde_json::Value
}

#[derive(Deserialize)]
enum SetupAction {
  #[serde(rename = "update")]
  Update,
  #[serde(rename = "setup")]
  Setup,
}

#[derive(Deserialize)]
pub(super) struct GithubAppRegistrationCallback {
  installation_id: i64,
  code: String,
  setup_action: SetupAction,
}

#[derive(Deserialize)]
pub(super) struct ListOfRepos {
  _total_count: i64,
  repositories: Vec<RepoInformation>,
}

#[derive(Deserialize)]
pub(super) struct DeploySite {
  domain: String,
  full_name: String,
}

pub(super) async fn github_setup_webhook(
  State(state): State<DoubleBlindState>,
  Query(query): Query<GithubAppRegistrationCallback>,
  jar: CookieJar,
) -> Result<(CookieJar, Redirect), Redirect> {
  const ERROR_REDIRECT: &str = "https://science.tanneberger.me/error";
  const SUCCESS_REDIRECT: &str = "https://science.tanneberger.me/project";

  let code = AuthorizationCode::new(query.code.clone());

  let token = state
    .oauth_github_client
    .exchange_code(code)
    .request_async(async_http_client)
    .await
    .map_err(|e| {
      error!("cannot exchange tokens with github error {e}");
      Redirect::to(ERROR_REDIRECT)
    })?;

  let access_token = token.access_token().secret().clone();
  let refresh_token = token
    .refresh_token()
    .ok_or(Redirect::to(ERROR_REDIRECT))?
    .secret()
    .to_string();

  let github_app_db = match state
    .project_service
    .create_github_app(
      query.installation_id,
      &refresh_token,
      OffsetDateTime::now_utc() + Duration::hours(8),
      &access_token,
      OffsetDateTime::now_utc() + Duration::days(6 * 30),
    )
    .await
  {
    Ok(value) => value,
    Err(e) => {
      error!("error when trying to create github app {e}");
      return Err(Redirect::to(ERROR_REDIRECT));
    }
  };

  let session_id = Uuid::new_v4();
  let session_data = SessionData {
    github_app: github_app_db.id,
  };

  state
    .sessions
    .write()
    .await
    .insert(session_id, Arc::new(session_data));

  let session_cookie = Cookie::build(SESSION_COOKIE, session_id.to_string())
    .domain("api.science.tanneberger.me")
    .same_site(SameSite::Lax)
    .path("/")
    .secure(true)
    .http_only(true)
    .max_age(Duration::days(1))
    .finish();

  Ok((jar.add(session_cookie), Redirect::to(SUCCESS_REDIRECT)))
}

pub async fn github_app_repositories(
  Session(session): Session,
  State(mut state): State<DoubleBlindState>,
  _jar: CookieJar,
) -> Result<Json<Vec<RepoInformation>>, StatusCode> {
  let mut github_app = match state
    .project_service
    .get_github_app(session.github_app)
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

  let client = Client::new();

  let response: ListOfRepos = client
    .get("https://api.github.com/installations/repositories")
    .header(reqwest::header::ACCEPT, "applggication/vnd.github+json")
    .header(
      reqwest::header::AUTHORIZATION,
      format!("Bearer {}", github_app.github_access_token.clone()),
    )
    .header("X-GitHub-Api-Version", "2022-11-28")
    .header(reqwest::header::USER_AGENT, "doubleblind-science")
    .json(&GithubDispatchEvent {
      event_type: "doubleblind-science-setup".to_string(),
    })
    .send()
    .await
    .map_err(|e| {
      error!("cannot fetch repositories that this app installation has access to {e}");
      StatusCode::INTERNAL_SERVER_ERROR
    })?
    .json::<ListOfRepos>()
    .await
    .map_err(|e| {
      error!("cannot deserialize list of repo response {e}");
      StatusCode::INTERNAL_SERVER_ERROR
    })?;

  Ok(Json(response.repositories))
}

pub async fn github_app_deploy_website(
  Session(session): Session,
  State(mut state): State<DoubleBlindState>,
  Json(data): Json<DeploySite>,
  _jar: CookieJar,
) -> Result<StatusCode, StatusCode> {
  let mut github_app = match state
    .project_service
    .get_github_app(session.github_app)
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
    .project_service
    .create_repository(github_app.id, data.domain, data.full_name.clone())
    .await
    .map_err(|e| {
      error!("cannot create repository {e}");
      StatusCode::INTERNAL_SERVER_ERROR
    })?;

  // triggering deployment via github webhook
  let client = Client::new();
  Ok(
    client
      .get(format!(
        "https://api.github.com/repos/{}/dispatches",
        &data.full_name
      ))
      .header(reqwest::header::ACCEPT, "application/vnd.github+json")
      .header(
        reqwest::header::AUTHORIZATION,
        format!("Bearer {}", github_app.github_access_token.clone()),
      )
      .header("X-GitHub-Api-Version", "2022-11-28")
      .header(reqwest::header::USER_AGENT, "doubleblind-science")
      .json(&GithubDispatchEvent {
        event_type: "doubleblind-science-setup".to_string(),
      })
      .send()
      .await
      .map_err(|e| {
        error!("cannot dispatch webhook event with github {e}");
        StatusCode::INTERNAL_SERVER_ERROR
      })?
      .status(),
  )
}
