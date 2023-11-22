use crate::auth::Session;
use crate::state::DoubleBlindState;
use axum::extract::State;
use tracing::error;
use entity::project;


pub(super) async fn user_projects(
  Session(session): Session,
  State(state): State<DoubleBlindState>,
) -> Vec<project::Model> {
  return match state
    .project_service
    .get_user_projects(session.user_id)
    .await {
    Ok(value) => value,
    Err(e) => {
      error!("error while querying projects {:?}", e);
      Vec::new()
    }
  }
}

pub(super) async fn project_name_available(
  Session(session): Session,
  State(state): State<DoubleBlindState>,
) -> bool {
  false
}

