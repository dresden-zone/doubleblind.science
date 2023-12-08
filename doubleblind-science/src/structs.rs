use serde::Deserialize;

#[derive(Deserialize)]
pub struct GithubUserInfo {
  pub id: i64,
}
