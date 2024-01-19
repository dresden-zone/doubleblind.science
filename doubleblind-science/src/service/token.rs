use axum::response::Response;
use chrono::prelude::*;
use chrono::Duration;
use core::result::Result;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use jwt_simple::prelude::{Deserialize, Serialize};
use reqwest::Client;
use std::collections::HashMap;
use std::error::Error;
use std::path::{Path, PathBuf};
use time::OffsetDateTime;

#[derive(Serialize)]
struct Claims {
  iss: String,
  iat: i64,
  exp: i64,
}

#[derive(Clone)]
pub struct TokenService {
  jwt: String,
}

#[derive(Serialize)]
struct RequestAccessTokens {
  repositories: Vec<String>,
  permissions: HashMap<&'static str, &'static str>,
}

#[derive(Deserialize)]
pub struct ResponseAccessTokens {
  pub token: String,
  pub expires_at: OffsetDateTime,
}

impl TokenService {
  pub fn new(client_id: String, private_key_file: &Path) -> TokenService {
    let private_key = std::fs::read_to_string(private_key_file).expect("cannot read private key");
    let jwt = TokenService::make_jwt(client_id, private_key).expect("cannot create jwt");

    TokenService { jwt }
  }
  pub fn make_jwt(client_id: String, private_key: String) -> Result<String, Box<dyn Error>> {
    let mut header = Header::new(Algorithm::RS256);
    header.typ = Some("JWT".to_string());

    let now = Local::now();
    let iat = now.timestamp();
    let exp = (now + Duration::hours(1)).timestamp();

    let claims = Claims {
      iss: client_id.clone(),
      iat,
      exp,
    };

    let jwt = encode(
      &header,
      &claims,
      &EncodingKey::from_rsa_pem(private_key.as_bytes())?,
    )?;
    return Ok(jwt);
  }

  pub async fn fetch_access_tokens_repo(
    &self,
    installation_id: i64,
    repositories: Vec<String>,
  ) -> anyhow::Result<ResponseAccessTokens> {
    let client = Client::new();

    Ok(
      client
        .post(format!(
          "api.github.com/app/installation/{installation_id}/access_tokens"
        ))
        .bearer_auth(&self.jwt)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .json(&RequestAccessTokens {
          repositories,
          permissions: HashMap::from([("repository_hooks", "write"), ("content", "read")]),
        })
        .send()
        .await?
        .error_for_status()?
        .json::<ResponseAccessTokens>()
        .await?,
    )
  }
}
