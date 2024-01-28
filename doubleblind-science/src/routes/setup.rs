use axum::http::HeaderMap;
use axum::{
  debug_handler,
  extract::{Json, Query, State},
  http::StatusCode,
  response::Redirect,
};
use axum_extra::extract::{
  cookie::{Cookie, SameSite},
  CookieJar,
};

use jwt_simple::algorithms::MACLike;

use entity::repository::Model;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use time::Duration;
use tracing::{error, info};
use uuid::Uuid;

use crate::auth::{Session, SessionData, SESSION_COOKIE};
use crate::routes::RepoInformation;
use crate::service::token::ResponseAccessTokens;
use crate::state::DoubleBlindState;

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
  github_id: i64,
}

#[derive(Deserialize)]
pub(super) struct InstallationInformation {
  id: i64,
}

#[derive(Deserialize)]
pub(super) struct GithubWebhookSetup {
  installation: InstallationInformation,
  repositories_added: Vec<RepoInformation>,
  repositories_removed: Vec<RepoInformation>,
}

pub(super) async fn github_forward_user(
  State(state): State<DoubleBlindState>,
  Query(query): Query<GithubAppRegistrationCallback>,
  _headers: HeaderMap,
) -> Result<(CookieJar, Redirect), Redirect> {
  const ERROR_REDIRECT: &str = "https://science.tanneberger.me/error";
  const SUCCESS_REDIRECT: &str = "https://science.tanneberger.me/projects";

  let session_id = Uuid::new_v4();

  let session_data = SessionData {
    installation_id: query.installation_id,
  };

  let result = state
    .sessions
    .write()
    .await
    .insert(session_id, Arc::new(session_data));

  info!("added session with session id: {session_id}, {:?}", state.sessions.read().await.get(&session_id));

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
pub(super) async fn github_create_installation(
  State(state): State<DoubleBlindState>,
  _headers: HeaderMap,
  raw_body: String,
) -> Result<StatusCode, StatusCode> {
  info!("setup new github project");
  // TODO: do HMAC Challange this endpoint should only be called from github
  // parsing json body from github
  let parsed_request: GithubWebhookSetup = serde_json::from_str(&raw_body).map_err(|e| {
    error!(
      "cannot parse request body from github {} {:?}",
      &raw_body, e
    );
    StatusCode::BAD_REQUEST
  })?;

  // look which repositories are already known
  let already_installed_repos: Vec<RepoInformation> = match state
    .project_service
    .all_repos_for_installation_id(parsed_request.installation.id)
    .await
    .map_err(|e| {
      error!("error all repos with this installation id {e}");

      StatusCode::INTERNAL_SERVER_ERROR
    })? {
    Some(values) => values
      .into_iter()
      .map(|x| RepoInformation {
        id: x.github_id,
        short_name: x.github_short_name,
        full_name: x.github_full_name,
      })
      .collect(),
    None => {
      info!("no values previous installed repos");
      Vec::new()
    }
  };

  // here we basically calculate (Known + New) - Removed
  let mut set_of_repos: HashSet<RepoInformation> =
    HashSet::from_iter(already_installed_repos.into_iter());

  for added_repo in parsed_request.repositories_added {
    set_of_repos.insert(added_repo);
  }

  for removed_repo in parsed_request.repositories_removed {
    set_of_repos.remove(&removed_repo);
  }

  let repos_with_permissions: Vec<RepoInformation> = set_of_repos.into_iter().collect();

  // get a new access token for this set of repos
  let access_token: ResponseAccessTokens = state
    .token_service
    .fetch_access_tokens_repo(
      parsed_request.installation.id,
      repos_with_permissions
        .iter()
        .map(|x| x.short_name.clone())
        .collect(),
    )
    .await
    .map_err(|e| {
      error!("error while trying to fetch access token {e}");

      StatusCode::INTERNAL_SERVER_ERROR
    })?;

  // create if github_app doesn't exist yet
  let github_app_db = match state
    .project_service
    .create_github_app(
      parsed_request.installation.id,
      &access_token.token,
      access_token.expires_at,
    )
    .await
  {
    Ok(value) => value,
    Err(e) => {
      error!("error when trying to create github app {e}");

      return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
  };

  // rewrite the list of repositories connected to github app
  state
    .project_service
    .rewrite_list_of_repositories(github_app_db.id, repos_with_permissions)
    .await
    .map_err(|e| {
      error!("error while trying to rewrite repo list {e}");

      StatusCode::INTERNAL_SERVER_ERROR
    })?;

  Ok(StatusCode::OK)
}

pub async fn github_app_repositories(
  Session(session): Session,
  State(state): State<DoubleBlindState>,
  _jar: CookieJar,
) -> Result<Json<Vec<RepoInformation>>, StatusCode> {
  match state
    .project_service
    .all_repos_for_installation_id(session.installation_id)
    .await
  {
    Ok(Some(value)) => Ok(Json(
      value
        .iter()
        .map(|x| RepoInformation {
          id: x.github_id,
          short_name: x.github_short_name.clone(),
          full_name: x.github_full_name.clone(),
        })
        .collect::<Vec<RepoInformation>>(),
    )),
    Ok(None) => {
      info!("no github installation with this name!");
      Err(StatusCode::NOT_FOUND)
    }
    Err(e) => {
      error!("error while trying to query github apps {e}");
      Err(StatusCode::INTERNAL_SERVER_ERROR)
    }
  }
}

pub async fn github_app_deploy_website(
  Session(session): Session,
  State(state): State<DoubleBlindState>,
  _jar: CookieJar,
  Json(data): Json<DeploySite>,
) -> Result<StatusCode, StatusCode> {
  let github_app = match state
    .project_service
    .get_github_app(session.installation_id)
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

  let repo = match state
    .project_service
    .deploy_repo(data.github_id.clone(), data.domain)
    .await
    .map_err(|e| {
      error!("cannot create repository {e}");
      StatusCode::INTERNAL_SERVER_ERROR
    })?
    .get(0)
  {
    None => {
      return Err(StatusCode::NOT_FOUND);
    }
    Some(value) => value.clone(),
  };

  // triggering deployment via github webhook
  let client = Client::new();
  Ok(
    client
      .get(format!(
        "https://api.github.com/repos/{}/dispatches",
        &repo.github_full_name
      ))
      .header(reqwest::header::ACCEPT, "application/vnd.github+json")
      .bearer_auth(github_app.github_access_token.clone())
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
