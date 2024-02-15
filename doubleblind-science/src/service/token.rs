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
use tracing::info;

#[derive(Clone)]
pub struct TokenService {
  jwt: String,
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
    let private_key = std::fs::read_to_string(private_key_file).expect("cannot read private key");
    let jwt = TokenService::make_jwt(client_id, private_key).expect("cannot create jwt");

    TokenService { jwt }
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
    Ok(encode_with_signer(&payload, &header, &signer)?)
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

    println!("DEBUG {:?} \nJWT {}", &request_body, &self.jwt);
    let temporary =       client
        .post(format!(
          "https://api.github.com/app/installations/{installation_id}/access_tokens"
        ))
        .bearer_auth(&self.jwt)
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .header(reqwest::header::USER_AGENT, "doubleblind-science")
        .json(&request_body)
        .send()
        .await?;

    info!("Response: {:#?}", &temporary);
    Ok(
        temporary
        .error_for_status()?
        .json::<ResponseAccessTokens>()
        .await?,
    )
  }
}
