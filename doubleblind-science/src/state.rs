use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use oauth2::basic::BasicClient;
use oauth2::{AuthUrl, ClientId, ClientSecret, CsrfToken, RedirectUrl, TokenUrl};
use sea_orm::{ConnectOptions, Database};
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

use migration::{Migrator, MigratorTrait};

use crate::auth::SessionData;
use crate::service::deployment::DeploymentService;
use crate::service::projects::ProjectService;
use crate::service::user::UserService;

#[derive(Clone)]
pub(crate) struct DoubleBlindState {
  pub oauth_github_client: BasicClient,
  pub csrf_state: Arc<Mutex<HashMap<Uuid, CsrfToken>>>,
  pub sessions: Arc<RwLock<HashMap<Uuid, Arc<SessionData>>>>,
  pub user_service: UserService,
  pub project_service: ProjectService,
  pub deployment_service: DeploymentService,
}

impl DoubleBlindState {
  pub async fn new(
    username: &str,
    password_file: &Path,
    host: &str,
    database: &str,
    github_client_id: &str,
    github_client_secret_path: &Path,
    website_path: &Path,
    website_domain: &str,
  ) -> DoubleBlindState {
    // reading secrets from files
    let database_password = std::fs::read_to_string(password_file)
      .unwrap_or_else(|_| panic!("cannot read password file: {:?}", &password_file));
    let github_client_secret =
      std::fs::read_to_string(github_client_secret_path).expect("cannot read github secret file");

    let mut db_options = ConnectOptions::new(format!(
      "postgresql://{}:{}@{}/{}",
      username, database_password, host, database
    ));
    db_options
      .max_connections(100)
      .min_connections(5)
      .connect_timeout(Duration::from_secs(8))
      .acquire_timeout(Duration::from_secs(8))
      .idle_timeout(Duration::from_secs(8))
      .max_lifetime(Duration::from_secs(8))
      .sqlx_logging(false);

    let db = Arc::new(
      Database::connect(db_options)
        .await
        .expect("cannot connect to postgres"),
    );
    Migrator::up(&*db, None)
      .await
      .expect("cannot run migrations");

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
      RedirectUrl::new("https://api.science.tanneberger.me/auth/callback/github".to_string())
        .expect("Invalid redirect URL"),
    );

    DoubleBlindState {
      oauth_github_client: client,
      csrf_state: Default::default(),
      sessions: Default::default(),
      user_service: UserService::from_db(db.clone()),
      project_service: ProjectService::from_db(db.clone()),
      deployment_service: DeploymentService::new(
        website_path.to_path_buf(),
        website_domain.to_string(),
      ),
    }
  }
}
