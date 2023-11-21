use serde::Deserialize;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(FromRow)]
pub struct GithubUser {
    id: Uuid,
    refresh_token: String,
}

#[derive(Deserialize)]
pub struct GithubUserInfo {
    pub id: i64,
}
