use chrono::prelude::*;
use chrono::Duration;
use core::result::Result;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use jwt_simple::prelude::{Deserialize, RS256KeyPair, RSAKeyPairLike, Serialize, UnixTimeStamp};
use reqwest::Client;
use std::collections::HashMap;
use std::error::Error;
use std::path::Path;
use jwt_simple::claims::JWTClaims;
use time::OffsetDateTime;

#[derive(Serialize)]
struct Claims {
  iss: i64,
  iat: i64,
  exp: i64,
}

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
    let exp = (now + Duration::minutes(8)).timestamp();

    let claims = JWTClaims{
      issued_at: Some(UnixTimeStamp::from_secs(iat as u64)),
      expires_at: Some(UnixTimeStamp::from_secs(exp as u64)),
      invalid_before: None,
      issuer: Some(client_id),
      subject: None,
      audiences: None,
      jwt_id: None,
      nonce: None,
      custom: (),
    };

    Ok(RS256KeyPair::from_pem(&private_key)?.sign(claims)?)

    //let jwt = encode(
    //  &header,
    //  &claims,
    //  &EncodingKey::from_rsa_pem(&private_key)?,
    //)?;

    //Ok(jwt)
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
    Ok(
      client
        .post(format!(
          "https://api.github.com/app/installations/{installation_id}/access_tokens"
        ))
        .bearer_auth(&self.jwt)
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .header(reqwest::header::USER_AGENT, "doubleblind-science")
        .json(&request_body)
        .send()
        .await?
        .error_for_status()?
        .json::<ResponseAccessTokens>()
        .await?,
    )
  }
}
