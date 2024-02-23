use std::collections::HashSet;
use std::sync::Arc;

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
use hmac::{Hmac, Mac};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use time::Duration;
use tracing::{error, info};
use uuid::Uuid;

use crate::auth::{Session, SessionData, SESSION_COOKIE};
use crate::routes::GithubRepoInformation;
use crate::service::deploy::DeploymentInformation;
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

#[derive(Deserialize, Debug)]
pub(super) struct GithubAppRegistrationCallback {
  installation_id: i64,
  //code: String,
  //setup_action: String,
}

#[derive(Deserialize)]
pub(super) struct DeploySite {
  domain: String,
  branch: String,
  github_id: i64,
}

#[derive(Deserialize)]
pub(super) struct InstallationInformation {
  id: i64,
}

#[derive(Deserialize)]
pub(super) struct GithubWebhookSetup {
  installation: InstallationInformation,
  repositories_added: Vec<GithubRepoInformation>,
  repositories_removed: Vec<GithubRepoInformation>,
}

#[derive(Serialize)]
pub(super) struct FrontendRepoInformation {
  pub id: i64,
  #[serde(rename = "name")]
  pub short_name: String,
  pub full_name: String,
  pub deployed: bool,
  pub domain: Option<String>,
  pub branch: Option<String>,
}

#[derive(Deserialize)]
pub(super) struct CommitObject {
  sha: String,
}

#[derive(Deserialize)]
pub(super) struct GithubCommit {
  r#ref: String,
  object: CommitObject,
}

pub(super) async fn github_forward_user(
  State(state): State<DoubleBlindState>,
  Query(query): Query<GithubAppRegistrationCallback>,
  _headers: HeaderMap,
) -> Result<(CookieJar, Redirect), Redirect> {
  const SUCCESS_REDIRECT: &str = "https://science.tanneberger.me/projects";

  let session_id = Uuid::new_v4();

  let session_data = SessionData {
    installation_id: query.installation_id,
  };

  let _result = state
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

pub(super) async fn github_create_installation(
  State(mut state): State<DoubleBlindState>,
  headers: HeaderMap,
  raw_body: String,
) -> Result<StatusCode, StatusCode> {
  info!("setup new github project");

  type HmacSha256 = Hmac<Sha256>;

  let hash = match headers.get("X-Hub-Signature-256") {
    Some(value) => value,
    None => {
      error!("github didn't send HMAC challange hash!");
      return Err(StatusCode::BAD_REQUEST);
    }
  };

  let hash_value = hash.to_str().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.split_at(7);

  match HmacSha256::new_from_slice(state.github_hmac_secret.as_ref()) {
    Ok(mut mac) => {
      mac.update(raw_body.as_ref());

      info!("validating slice: {}", &hash_value.1);
      match mac.verify_slice((hash_value.1).as_ref()) {
        Ok(_) => {

        },
        Err(e) => {
          error!("non github entity tried to call the webhook endpoint! {e}");
          return Err(StatusCode::FORBIDDEN);
        }
      }

    }
    Err(e) => {
      error!("cannot generate hmac with error {}", e);
      return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
  }

  // parsing json body from github
  let parsed_request: GithubWebhookSetup = serde_json::from_str(&raw_body).map_err(|e| {
    error!(
      "cannot parse request body from github {} {:?}",
      &raw_body, e
    );
    StatusCode::BAD_REQUEST
  })?;

  // look which repositories are already known
  let already_installed_repos: Vec<GithubRepoInformation> = match state
    .project_service
    .all_repos_for_installation_id(parsed_request.installation.id)
    .await
    .map_err(|e| {
      error!("error all repos with this installation id {e}");
      StatusCode::INTERNAL_SERVER_ERROR
    })? {
    Some(values) => values
      .into_iter()
      .map(|x| GithubRepoInformation {
        id: x.github_id,
        short_name: x.github_short_name,
        full_name: x.github_full_name,
        deployed: x.deployed,
        domain: x.domain,
        branch: x.branch,
        last_update: x.last_update,
      })
      .collect(),
    None => {
      info!("no values previous installed repos");
      Vec::new()
    }
  };

  // here we basically calculate (Known + New) - Removed
  let mut set_of_repos: HashSet<GithubRepoInformation> =
    HashSet::from_iter(already_installed_repos.into_iter());

  for added_repo in parsed_request.repositories_added {
    info!("(+) Repo {} Added!", &added_repo.full_name);
    set_of_repos.insert(added_repo);
  }

  for removed_repo in parsed_request.repositories_removed {
    info!("(-) Repo {} Removed!", &removed_repo.full_name);
    set_of_repos.remove(&removed_repo);
  }

  let repos_with_permissions: Vec<GithubRepoInformation> = set_of_repos.into_iter().collect();

  state.repos_per_installation.insert(
    parsed_request.installation.id,
    repos_with_permissions.clone(),
  );

  // create if github_app doesn't exist yet
  let github_app_db = match state
    .project_service
    .create_github_app(parsed_request.installation.id)
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
  State(mut state): State<DoubleBlindState>,
  _jar: CookieJar,
) -> Result<Json<Vec<FrontendRepoInformation>>, StatusCode> {
  if let Some(values) = state
    .repos_per_installation
    .remove(&session.installation_id)
  {
    return Ok(Json(
      values
        .iter()
        .map(|x| FrontendRepoInformation {
          id: x.id,
          short_name: x.short_name.clone(),
          full_name: x.full_name.clone(),
          deployed: x.deployed,
          branch: x.branch.clone(),
          domain: x.domain.clone(),
        })
        .collect::<Vec<FrontendRepoInformation>>(),
    ));
  }

  match state
    .repos_per_installation
    .remove(&session.installation_id)
  {
    Some(values) => {
      return Ok(Json(
        values
          .iter()
          .map(|x| FrontendRepoInformation {
            id: x.id,
            short_name: x.short_name.clone(),
            full_name: x.full_name.clone(),
            deployed: x.deployed,
            branch: x.branch.clone(),
            domain: x.domain.clone(),
          })
          .collect::<Vec<FrontendRepoInformation>>(),
      ));
    }
    None => {}
  }

  match state
    .project_service
    .all_repos_for_installation_id(session.installation_id)
    .await
  {
    Ok(Some(value)) => Ok(Json(
      value
        .iter()
        .map(|x| FrontendRepoInformation {
          id: x.github_id,
          short_name: x.github_short_name.clone(),
          full_name: x.github_full_name.clone(),
          deployed: x.deployed,
          branch: x.branch.clone(),
          domain: x.domain.clone(),
        })
        .collect::<Vec<FrontendRepoInformation>>(),
    )),
    Ok(None) => {
      info!(
        "no github installation with this name! installation_id: {}",
        &session.installation_id
      );
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
  State(mut state): State<DoubleBlindState>,
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
      info!(
        "no github installation with this name! installation_id: {}",
        &session.installation_id
      );
      return Err(StatusCode::NOT_FOUND);
    }
    Err(e) => {
      error!("error while trying to query github apps {e}");
      return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
  };

  let repo = match state
    .project_service
    .deploy_repo(data.github_id, data.domain.clone(), data.branch.clone())
    .await
    .map_err(|e| {
      error!("cannot create repository {e}");
      StatusCode::INTERNAL_SERVER_ERROR
    })?
    .first()
  {
    None => {
      return Err(StatusCode::NOT_FOUND);
    }
    Some(value) => value.clone(),
  };

  let access_token: ResponseAccessTokens = state
    .token_service
    .fetch_access_tokens_repo(github_app.installation_id, vec![repo.github_short_name])
    .await
    .map_err(|e| {
      error!("error while trying to fetch access token {e}");
      StatusCode::INTERNAL_SERVER_ERROR
    })?;

  let client = Client::new();
  client
    .post(format!(
      "https://api.github.com/repos/{}/hooks",
      &repo.github_full_name
    ))
    .header(reqwest::header::ACCEPT, "application/vnd.github+json")
    .bearer_auth(&access_token.token)
    .header("X-GitHub-Api-Version", "2022-11-28")
    .header(reqwest::header::USER_AGENT, "doubleblind-science")
    .json(&WebhookRegistrationRequest {
      name: "web".to_string(),
      active: true,
      events: vec!["push".to_string()],
      config: WebHookInformation {
        url: "https://api.science.tanneberger.me/v1/github/hooks/deploy".to_string(),
        content_type: "json".to_string(),
        insecure_ssl: "0".to_string(),
      },
    })
    .send()
    .await
    .map(|response| {
      info!("Response for Hook Creation {:#?}", response);
      response
    })
    .map_err(|e| {
      error!("cannot dispatch webhook event with github {e}");
      StatusCode::INTERNAL_SERVER_ERROR
    })?
    .status();

  let git_refs: Vec<GithubCommit> = client
    .get(format!(
      "https://api.github.com/repos/{}/git/refs",
      &repo.github_full_name
    ))
    .header(reqwest::header::ACCEPT, "application/vnd.github+json")
    .bearer_auth(&access_token.token)
    .header("X-GitHub-Api-Version", "2022-11-28")
    .header(reqwest::header::USER_AGENT, "doubleblind-science")
    .send()
    .await
    .map(|response| {
      info!("Response for Hook Creation {:#?}", response);
      response
    })
    .map_err(|e| {
      error!("cannot dispatch webhook event with github {e}");
      StatusCode::INTERNAL_SERVER_ERROR
    })?
    .json::<Vec<GithubCommit>>()
    .await
    .map_err(|e| {
      error!("cannot parse git commit response {e}");
      StatusCode::INTERNAL_SERVER_ERROR
    })?;

  let commit_ref = match git_refs
    .iter()
    .find(|p| p.r#ref == format!("refs/heads/{}", &data.branch))
  {
    Some(value) => value,
    None => {
      error!("branch not found {}", &data.branch);
      return Err(StatusCode::NOT_FOUND);
    }
  };

  state
    .deployment_service
    .queue_deployment(DeploymentInformation {
      full_name: repo.github_full_name,
      token: access_token.token,
      domain: data.domain,
      commit_id: commit_ref.object.sha.clone(),
    })
    .await
    .map_err(|_e| {
      error!("queueing for deployment failed!");
      StatusCode::INTERNAL_SERVER_ERROR
    })?;

  Ok(StatusCode::OK)
  // triggering deployment via github webhook
}
