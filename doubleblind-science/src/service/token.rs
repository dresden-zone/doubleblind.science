use anyhow::anyhow;
use core::result::Result;
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;
use std::time::SystemTime;

use josekit::jws::{JwsHeader, RS256};
use josekit::jwt::{encode_with_signer, JwtPayload};
use josekit::JoseError;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Clone)]
pub struct TokenService {
  client_id: String,
  secret: String,
}

#[derive(Serialize, Debug)]
struct RequestAccessTokens {
  repositories: Vec<String>,
  permissions: HashMap<&'static str, &'static str>,
}

#[derive(Deserialize)]
pub struct ResponseAccessTokens {
  pub token: String,
  #[serde(with = "time::serde::iso8601")]
  pub expires_at: OffsetDateTime,
}

impl TokenService {
  pub fn new(client_id: String, private_key_file: &Path) -> TokenService {
    let secret = std::fs::read_to_string(private_key_file).expect("cannot read private key");

    TokenService { secret, client_id }
  }

  pub fn make_jwt(client_id: String, private_key: String) -> Result<String, JoseError> {
    let mut header = JwsHeader::new();
    header.set_token_type("JWT");

    let now = SystemTime::now() - Duration::from_secs(10);
    let expires = now + Duration::from_secs(60 * 8 + 10);

    let mut payload = JwtPayload::new();
    payload.set_issued_at(&now);
    payload.set_expires_at(&expires);
    payload.set_issuer(client_id);

    let signer = RS256.signer_from_pem(private_key)?;
    encode_with_signer(&payload, &header, &signer)
  }

  pub async fn fetch_access_tokens_repo(
    &self,
    installation_id: i64,
    repositories: Vec<String>,
  ) -> anyhow::Result<ResponseAccessTokens> {
    let client = Client::new();
    let request_body = &RequestAccessTokens {
      repositories,
      permissions: HashMap::from([("repository_hooks", "write"), ("contents", "read")]),
    };

    let jwt = TokenService::make_jwt(self.client_id.clone(), self.secret.clone()).expect("cannot create jwt");

    println!("DEBUG {:?} \nJWT {}", &request_body, &jwt);
    let temporary = client
      .post(format!(
        "https://api.github.com/app/installations/{installation_id}/access_tokens"
      ))
      .bearer_auth(&jwt)
      .header(reqwest::header::ACCEPT, "application/vnd.github+json")
      .header("X-GitHub-Api-Version", "2022-11-28")
      .header(reqwest::header::USER_AGENT, "doubleblind-science")
      .json(&request_body)
      .send()
      .await?;

    if temporary.status().is_success() {
      Ok(temporary.json().await?)
    } else {
      Err(anyhow!(
        "Error while obtaining token: {} {}",
        temporary.status(),
        temporary.text().await?
      ))
    }
  }
}
