use sqlx::FromRow;
use uuid::Uuid;

#[derive(FromRow)]
pub struct GithubUser {
    id: Uuid,
    refresh_token: String,
}
