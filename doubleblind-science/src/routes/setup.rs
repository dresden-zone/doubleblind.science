use axum::http::HeaderMap;
use axum::{
  extract::{Json, Query, State},
  http::StatusCode,
  response::Redirect,
};
use axum_extra::extract::{
  cookie::{Cookie, SameSite},
  CookieJar,
};
use clap::builder::styling::AnsiColor::Red;
use entity::prelude::Repository;
use jwt_simple::algorithms::{HS256Key, MACLike};
use jwt_simple::claims::Claims;
use oauth2::{reqwest::async_http_client, AuthorizationCode, TokenResponse};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use time::{Duration, OffsetDateTime};
use tracing::{error, info};
use uuid::Uuid;

use crate::auth::{Session, SessionData, SESSION_COOKIE};
use crate::service::token::ResponseAccessTokens;
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

#[derive(Deserialize, Debug)]
enum SetupAction {
  #[serde(rename = "update")]
  Update,
  #[serde(rename = "setup")]
  Setup,
  #[serde(rename = "removed")]
  Removed,
}

#[derive(Deserialize, Debug)]
pub(super) struct GithubAppRegistrationCallback {
  installation_id: i64,
  code: String,
  setup_action: String,
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

#[derive(Deserialize)]
pub(super) struct GithubWebhookSetup {
  repositories_added: Vec<RepoInformation>,
  repositories_removed: Vec<RepoInformation>,
}

pub(super) async fn github_setup_webhook(
  State(state): State<DoubleBlindState>,
  Query(query): Query<GithubAppRegistrationCallback>,
  headers: HeaderMap,
  raw_body: String,
) -> Result<(CookieJar, Redirect), Redirect> {
  const ERROR_REDIRECT: &str = "https://science.tanneberger.me/error";
  const SUCCESS_REDIRECT: &str = "https://science.tanneberger.me/project";

  info!("setup new github project {:?}", &query);

  // parsing json body from github
  let parsed_request: GithubWebhookSetup =
    serde_json::from_str(&raw_body).map_err(|_| Redirect::to(ERROR_REDIRECT))?;

  // look which repositories are already known
  let already_installed_repos: Vec<String> = match state
    .project_service
    .all_repos_for_installation_id(query.installation_id)
    .await
    .map_err(|_| Redirect::to(ERROR_REDIRECT))?
  {
    Some(values) => values.into_iter().map(|x| x.github_name).collect(),
    None => {
      return Err(Redirect::to(ERROR_REDIRECT));
    }
  };

  // here we basically calculate (Known + New) - Removed
  let mut set_of_repos: HashSet<String> = HashSet::from_iter(already_installed_repos.into_iter());

  for added_repo in parsed_request.repositories_added {
    set_of_repos.insert(added_repo.name);
  }

  for removed_repo in parsed_request.repositories_removed {
    set_of_repos.remove(&*removed_repo.name);
  }

  let repos_with_permissions: Vec<String> = set_of_repos.into_iter().collect();

  // get a new access token for this set of repos
  let access_token: ResponseAccessTokens = state
    .token_service
    .fetch_access_tokens_repo(query.installation_id, repos_with_permissions.clone())
    .await
    .map_err(|_| Redirect::to(ERROR_REDIRECT))?;

  // create if github_app doesn't exist yet
  let github_app_db = match state
    .project_service
    .create_github_app(
      query.installation_id,
      &access_token.token,
      access_token.expires_at,
    )
    .await
  {
    Ok(value) => value,
    Err(e) => {
      error!("error when trying to create github app {e}");
      return Err(Redirect::to(ERROR_REDIRECT));
    }
  };

  // rewrite the list of repositories connected to github app
  state
    .project_service
    .rewrite_list_of_repositories(github_app_db.id, repos_with_permissions)
    .await
    .map_err(|_| Redirect::to(ERROR_REDIRECT))?;

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

  let jar = CookieJar::new();

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

  state
    .project_service
    .deploy_repo(data.full_name.clone(), data.domain)
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
