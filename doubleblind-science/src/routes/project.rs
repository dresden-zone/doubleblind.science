use axum::debug_handler;
use axum::extract::{Json, State};
use axum::http::StatusCode;
use entity::project;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use tracing::{error, info};

use crate::auth::Session;
use crate::state::DoubleBlindState;

#[derive(Deserialize)]
pub(super) struct CreateProjectRequest {
  pub name: String,
  pub repo: i64,
}

#[derive(Serialize, Deserialize)]
pub(super) struct RepoInformation {
  id: i64,
  name: String,
  full_name: String,
}

pub(super) async fn user_projects(
  Session(session): Session,
  State(state): State<DoubleBlindState>,
) -> Vec<project::Model> {
  match state
    .project_service
    .get_user_projects(session.user_id)
    .await
  {
    Ok(value) => value,
    Err(e) => {
      error!("error while querying projects {:?}", e);
      Vec::new()
    }
  }
}

pub(super) async fn user_repos(
  Session(session): Session,
  State(mut state): State<DoubleBlindState>,
) -> Result<Json<Vec<RepoInformation>>, StatusCode> {
  let user_info = match state.user_service.get_user(session.user_id).await {
    Ok(Some(user)) => user,
    Err(e) => {
      error!("while trying to fetch user {:?}", e);
      return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
    _ => {
      return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
  };

  if let (
    Some(mut access_token),
    Some(access_token_expr),
    Some(refresh_token),
    Some(refresh_token_expr),
  ) = (
    user_info.github_access_token,
    user_info.github_access_token_expire,
    user_info.github_refresh_token,
    user_info.github_refresh_token_expire,
  ) {
    if access_token_expr > OffsetDateTime::now_utc() {
      match state
        .user_service
        .fresh_access_token(&mut state.oauth_github_client, session.user_id)
        .await
      {
        Some(new_token) => {
          access_token = new_token;
        }
        None => {
          return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
      };
    }

    Ok(Json(
      reqwest::Client::new()
        .get("https://api.github.com/user/repos")
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .header(
          reqwest::header::AUTHORIZATION,
          format!("Bearer {}", access_token.clone()),
        )
        .header("X-GitHub-Api-Version", "2022-11-28")
        .header(reqwest::header::USER_AGENT, "doubleblind-science")
        .send()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .json()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    ))
  } else {
    Err(StatusCode::INTERNAL_SERVER_ERROR)
  }
}

pub(super) async fn create_project(
  Session(session): Session,
  State(state): State<DoubleBlindState>,
  Json(data): Json<CreateProjectRequest>,
) -> Result<Json<Option<project::Model>>, StatusCode> {
  if data.name.len() <= 6 {
    return Err(StatusCode::BAD_REQUEST);
  }

  match state
    .project_service
    .get_project_by_name_or_repo(&data.name, data.repo)
    .await
  {
    Ok(Some(found_project)) => {
      info!("project already exists name or repo");
      return Err(StatusCode::BAD_REQUEST);
    }
    Err(e) => {
      error!("error while searching for projects {:?}", e);
      return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
    _ => {}
  }

  let user_info = match state.user_service.get_user(session.user_id).await {
    Ok(Some(user)) => user,
    Err(e) => {
      error!("while trying to fetch user {:?}", e);
      return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
    _ => {
      return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
  };

  //let get_repo_info = reqwest::
  // - inserten

  Ok(Json(
    state
      .project_service
      .create_project(
        user_info.id,
        data.name.to_string(),
        "".to_string(),
        data.repo,
      )
      .await
      .map_err(|err| {
        error!("Error while inserting project: {:?}", err);
        StatusCode::INTERNAL_SERVER_ERROR
      })?,
  ))
}
