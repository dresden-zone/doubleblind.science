use sqlx::postgres::PgPoolOptions;
use sqlx::{Error, PgPool, Pool, Postgres};
use std::collections::HashMap;
use std::path::Path;

use oauth2::basic::BasicClient;
use oauth2::reqwest::async_http_client;
use oauth2::{
    AccessToken, AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, RedirectUrl, Scope,
    TokenResponse, TokenUrl,
};
use uuid::Uuid;

#[derive(Clone)]
pub(crate) struct DoubleBlindState {
    pub database: Pool<Postgres>,
    pub oauth_github_client: BasicClient,
    pub csrf_state: HashMap<Uuid, CsrfToken>,
    pub github_tokens: HashMap<Uuid, AccessToken>,
}

impl DoubleBlindState {
    pub async fn new(
        username: &str,
        password_file: &Path,
        host: &str,
        database: &str,
        github_client_id: &str,
        github_client_secret_path: &Path,
    ) -> DoubleBlindState {
        // reading secrets from files
        let database_password =
            std::fs::read_to_string(password_file).expect(format!("cannot read password file: {}", &password_file));
        let github_client_secret = std::fs::read_to_string(github_client_secret_path)
            .expect("cannot read github secret file");

        // opening connection to postgres
        let connection = PgPoolOptions::new()
            .max_connections(5)
            .connect(&*format!(
                "postgres://{}:{}@{}/{}",
                username, database_password, host, database
            ))
            .await
            .expect("cannot connect to database");

        // adding parsing information required to talk to github api
        let parsed_github_client_id = ClientId::new(github_client_id.to_string());
        let parsed_github_secret = ClientSecret::new(github_client_secret);

        // urls how to talk to github oauth
        let auth_url = AuthUrl::new("https://github.com/login/oauth/authorize".to_string())
            .expect("Invalid authorization endpoint URL");
        let token_url = TokenUrl::new("https://github.com/login/oauth/access_token".to_string())
            .expect("Invalid token endpoint URL");

        // Set up the config for the Github OAuth2 process.
        let client = BasicClient::new(
            parsed_github_client_id,
            Some(parsed_github_secret),
            auth_url,
            Some(token_url),
        )
        .set_redirect_uri(
            RedirectUrl::new("https://api.doubleblind.science/auth/callback/github".to_string())
                .expect("Invalid redirect URL"),
        );

        DoubleBlindState {
            database: connection,
            oauth_github_client: client,
            csrf_state: HashMap::new(),
            github_tokens: HashMap::new(),
        }
    }
}
