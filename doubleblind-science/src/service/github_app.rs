use oauth2::basic::BasicClient;
use oauth2::reqwest::async_http_client;
use oauth2::{RefreshToken, TokenResponse};
use std::sync::Arc;

use entity::github_app::Model;
use entity::{github_app, repository};
use sea_orm::entity::EntityTrait;
use sea_orm::ActiveValue::Unchanged;
use sea_orm::ColumnTrait;
use sea_orm::QueryFilter;

use sea_orm::Set;
use sea_orm::{ActiveModelTrait, DatabaseConnection};


use time::{Duration, OffsetDateTime};

use uuid::Uuid;

#[derive(Clone)]
pub(crate) struct ProjectService {
  db: Arc<DatabaseConnection>,
}

impl ProjectService {
  pub(crate) fn from_db(db: Arc<DatabaseConnection>) -> ProjectService {
    ProjectService { db }
  }

  pub(crate) async fn all_github_app_installations(
    &self,
  ) -> anyhow::Result<Vec<github_app::Model>> {
    Ok(github_app::Entity::find().all(&*self.db).await?)
  }
  pub(crate) async fn get_github_app(&self, id: Uuid) -> anyhow::Result<Option<github_app::Model>> {
    Ok(github_app::Entity::find_by_id(id).one(&*self.db).await?)
  }

  pub(crate) async fn create_github_app(
    &self,
    installation_id: i64,
    refresh_token: &String,
    refresh_token_expire: OffsetDateTime,
    access_token: &String,
    access_token_expire: OffsetDateTime,
  ) -> anyhow::Result<github_app::Model> {
    let _github_app_uuid = Uuid::new_v4();
    Ok(
      github_app::ActiveModel {
        id: Set(Uuid::new_v4()),
        installation_id: Set(installation_id),
        github_refresh_token: Set(refresh_token.clone()),
        github_refresh_token_expire: Set(refresh_token_expire),
        github_access_token: Set(access_token.clone()),
        github_access_token_expire: Set(access_token_expire),
        last_update: Default::default(),
      }
      .insert(&*self.db)
      .await?,
    )
  }

  pub(crate) async fn delete(&self, github_app_id: Uuid) -> anyhow::Result<bool> {
    Ok(
      github_app::Entity::delete_by_id(github_app_id)
        .exec(&*self.db)
        .await?
        .rows_affected
        > 0,
    )
  }

  pub(crate) async fn update_access_token(
    &self,
    model: Model,
    access_token: &String,
    access_token_expire: OffsetDateTime,
  ) -> anyhow::Result<Option<Model>> {
    Ok(Some(
      github_app::ActiveModel {
        id: Unchanged(model.id),
        installation_id: Unchanged(model.installation_id),
        github_refresh_token: Unchanged(model.github_refresh_token),
        github_refresh_token_expire: Unchanged(model.github_access_token_expire),
        github_access_token: Set(access_token.clone()),
        github_access_token_expire: Set(access_token_expire),
        last_update: Set(OffsetDateTime::now_utc()),
      }
      .update(&*self.db)
      .await?,
    ))
  }

  pub(crate) async fn update_refresh_token(
    &self,
    model: Model,
    refresh_token: &String,
    refresh_token_expire: OffsetDateTime,
  ) -> anyhow::Result<Option<Model>> {
    Ok(Some(
      github_app::ActiveModel {
        id: Unchanged(model.id),
        installation_id: Unchanged(model.installation_id),
        github_refresh_token: Set(refresh_token.clone()),
        github_refresh_token_expire: Set(refresh_token_expire),
        github_access_token: Unchanged(model.github_access_token),
        github_access_token_expire: Unchanged(model.github_access_token_expire),
        last_update: Set(OffsetDateTime::now_utc()),
      }
      .update(&*self.db)
      .await?,
    ))
  }

  pub(crate) async fn trust_github_app(&self, github_app_id: Uuid) -> anyhow::Result<bool> {
    let found_github_app: Option<entity::github_app::Model> =
      github_app::Entity::find_by_id(github_app_id)
        .one(&*self.db)
        .await?;
    if let Some(github_app) = found_github_app {
      github_app::ActiveModel {
        id: Unchanged(github_app_id),
        installation_id: Unchanged(github_app.installation_id),
        github_refresh_token: Unchanged(github_app.github_refresh_token),
        github_refresh_token_expire: Unchanged(github_app.github_refresh_token_expire),
        github_access_token: Unchanged(github_app.github_access_token),
        github_access_token_expire: Unchanged(github_app.github_refresh_token_expire),
        last_update: Set(OffsetDateTime::now_utc()),
      }
      .update(&*self.db)
      .await?;
      Ok(true)
    } else {
      Ok(false)
    }
  }
  pub(crate) async fn refresh_tokens(
    &mut self,
    mut github_app: Model,
    oauth_client: &mut BasicClient,
  ) -> anyhow::Result<Model> {
    if github_app.github_refresh_token_expire - Duration::days(30) < OffsetDateTime::now_utc()
      || github_app.github_access_token_expire - Duration::minutes(30) < OffsetDateTime::now_utc()
    {
      // updating refresh token

      let value = oauth_client
        .exchange_refresh_token(&RefreshToken::new(github_app.github_refresh_token.clone()))
        .request_async(async_http_client)
        .await?;

      let access_token = value.access_token().secret();
      let expire_access_token = OffsetDateTime::now_utc()
        + value
          .expires_in()
          .unwrap_or(core::time::Duration::from_secs(60 * 60 * 8));

      self
        .update_access_token(github_app.clone(), access_token, expire_access_token)
        .await?;

      github_app.github_access_token = access_token.clone();
      github_app.github_access_token_expire = expire_access_token;

      match value.refresh_token() {
        Some(refresh_token) => {
          let expire_refresh_token = OffsetDateTime::now_utc() + Duration::days(6 * 30);
          self
            .update_refresh_token(
              github_app.clone(),
              refresh_token.secret(),
              expire_refresh_token,
            )
            .await?;
          github_app.github_refresh_token = refresh_token.secret().clone();
          github_app.github_refresh_token_expire = expire_refresh_token;
        }
        _ => {}
      }
    }

    Ok(github_app)
  }

  pub(crate) async fn create_repository(
    &self,
    github_app: Uuid,
    domain: String,
    github_full_name: String,
  ) -> anyhow::Result<repository::Model> {
    let repo_uuid = Uuid::new_v4();
    Ok(
      repository::ActiveModel {
        id: Set(repo_uuid),
        github_app: Set(github_app),
        domain: Set(domain),
        github_name: Set(github_full_name),
        trusted: Set(false),
        created_at: Set(OffsetDateTime::now_utc()),
        last_update: Set(OffsetDateTime::now_utc()),
      }
      .insert(&*self.db)
      .await?,
    )
  }

  pub(crate) async fn get_repository(
    &self,
    github_full_name: String,
  ) -> anyhow::Result<Option<repository::Model>> {
    Ok(
      repository::Entity::find()
        .filter(repository::Column::GithubName.eq(github_full_name))
        .one(&*self.db)
        .await?,
    )
  }
}
