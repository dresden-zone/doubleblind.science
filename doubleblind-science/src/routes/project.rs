use crate::auth::Session;
use crate::state::DoubleBlindState;
use axum::extract::State;

pub(super) async fn project_available(
  Session(session): Session,
  State(state): State<DoubleBlindState>,
) {
  state
    .project_service
    .get_available_projects(session.user_id)
    .await;
}
