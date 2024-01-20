use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use sea_orm::{ConnectOptions, Database};
use tokio::sync::RwLock;
use uuid::Uuid;

use migration::{Migrator, MigratorTrait};

use crate::auth::SessionData;
use crate::service::deploy::DeploymentService;
use crate::service::github_app::ProjectService;
use crate::service::token::TokenService;

#[derive(Clone)]
pub(crate) struct DoubleBlindState {
  pub sessions: Arc<RwLock<HashMap<Uuid, Arc<SessionData>>>>,
  pub project_service: ProjectService,
  pub token_service: TokenService,
  pub deployment_service: DeploymentService,
  pub github_hmac_secret: String,
}

impl DoubleBlindState {
  pub async fn new(
    username: &str,
    password_file: &Path,
    host: &str,
    database: &str,
    github_client_id: &str,
    website_path: &Path,
    website_domain: &str,
    github_hmac_secret_file: &Path,
    github_private_key_file: &Path,
  ) -> DoubleBlindState {
    // reading secrets from files
    let database_password = std::fs::read_to_string(password_file)
      .unwrap_or_else(|_| panic!("cannot read password file: {:?}", &password_file));
    let github_hmac_secret = std::fs::read_to_string(github_hmac_secret_file)
      .expect("cannot read github hmac secret file");

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

    DoubleBlindState {
      sessions: Default::default(),
      project_service: ProjectService::from_db(db.clone()),
      deployment_service: DeploymentService::new(
        website_path.to_path_buf(),
        website_domain.to_string(),
      ),
      token_service: TokenService::new(github_client_id.to_string(), github_private_key_file),
      github_hmac_secret,
    }
  }
}
