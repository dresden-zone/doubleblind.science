use std::sync::Arc;
use std::time::Duration;

use oauth2::basic::BasicClient;
use oauth2::reqwest::async_http_client;
use oauth2::RefreshToken;
use oauth2::TokenResponse;
use sea_orm::entity::EntityTrait;
use sea_orm::ActiveValue::Unchanged;
use sea_orm::ColumnTrait;
use sea_orm::QueryFilter;
use sea_orm::Set;
use sea_orm::{ActiveModelTrait, DatabaseConnection};
use time::OffsetDateTime;
use tracing::error;
use uuid::Uuid;

use entity::user;

#[derive(Clone)]
pub(crate) struct UserService {
  db: Arc<DatabaseConnection>,
}

impl UserService {
  pub(crate) fn from_db(db: Arc<DatabaseConnection>) -> UserService {
    UserService { db }
  }

  pub(crate) async fn all_users(&mut self) -> anyhow::Result<Vec<user::Model>> {
    Ok(user::Entity::find().all(&*self.db).await?)
  }
  pub(crate) async fn get_user(&self, user_id: Uuid) -> anyhow::Result<Option<user::Model>> {
    Ok(user::Entity::find_by_id(user_id).one(&*self.db).await?)
  }

  pub(crate) async fn get_user_by_github(
    &mut self,
    github_id: i64,
  ) -> anyhow::Result<Option<user::Model>> {
    Ok(
      user::Entity::find()
        .filter(user::Column::GithubUserId.eq(github_id))
        .one(&*self.db)
        .await?,
    )
  }

  pub(crate) async fn create_github_user(
    &mut self,
    github_id: i64,
    github_refresh_token: String,
    github_refresh_token_expire: OffsetDateTime,
    github_access_token: String,
    github_access_token_expire: OffsetDateTime,
  ) -> anyhow::Result<user::Model> {
    let user_uuid = Uuid::new_v4();

    Ok(
      user::ActiveModel {
        id: Set(user_uuid),
        platform: Set(1),
        trusted: Set(false),
        admin: Set(false),
        github_user_id: Set(Some(github_id)),
        github_refresh_token: Set(Some(github_refresh_token)),
        github_refresh_token_expire: Set(Some(github_refresh_token_expire)),
        github_access_token: Set(Some(github_access_token)),
        github_access_token_expire: Set(Some(github_access_token_expire)),
        last_update: Set(OffsetDateTime::now_utc()),
      }
      .insert(&*self.db)
      .await?,
    )
  }

  pub(crate) async fn delete(&mut self, user_id: Uuid) -> anyhow::Result<bool> {
    Ok(
      user::Entity::delete_by_id(user_id)
        .exec(&*self.db)
        .await?
        .rows_affected
        > 0,
    )
  }

  pub(crate) async fn update(
    &mut self,
    user_id: Uuid,
    trusted: bool,
    admin: bool,
  ) -> anyhow::Result<bool> {
    let found_user = user::Entity::find_by_id(user_id).one(&*self.db).await?;

    if let Some(user) = found_user {
      user::ActiveModel {
        id: Unchanged(user_id),
        platform: Unchanged(user.platform),
        trusted: Set(trusted),
        admin: Set(admin),
        github_user_id: Set(user.github_user_id),
        github_refresh_token: Unchanged(user.github_refresh_token),
        github_refresh_token_expire: Set(user.github_access_token_expire),
        github_access_token: Set(user.github_access_token),
        github_access_token_expire: Set(user.github_access_token_expire),
        last_update: Set(OffsetDateTime::now_utc()),
      }
      .update(&*self.db)
      .await?;
      Ok(true)
    } else {
      Ok(false)
    }
  }

  pub(crate) async fn update_github_access_token(
    &mut self,
    user_id: Uuid,
    github_access_token: &str,
    github_access_token_expire: OffsetDateTime,
  ) -> anyhow::Result<bool> {
    let found_user = user::Entity::find_by_id(user_id).one(&*self.db).await?;

    if let Some(user) = found_user {
      user::ActiveModel {
        id: Unchanged(user_id),
        platform: Unchanged(user.platform),
        trusted: Unchanged(user.trusted),
        admin: Unchanged(user.admin),
        github_user_id: Unchanged(user.github_user_id),
        github_refresh_token: Set(user.github_refresh_token),
        github_refresh_token_expire: Set(user.github_refresh_token_expire),
        github_access_token: Set(Some(github_access_token.to_string())),
        github_access_token_expire: Set(Some(github_access_token_expire)),
        last_update: Set(OffsetDateTime::now_utc()),
      }
      .update(&*self.db)
      .await?;
      Ok(true)
    } else {
      Ok(false)
    }
  }

  pub async fn fresh_access_token(
    &mut self,
    oauth_client: &mut BasicClient,
    user_id: Uuid,
  ) -> Option<String> {
    if let Ok(Some(user_info)) = self.get_user(user_id).await {
      if let (
        Some(access_token),
        Some(access_token_expr),
        Some(refresh_token),
        Some(refresh_token_expr),
      ) = (
        user_info.github_access_token,
        user_info.github_access_token_expire,
        user_info.github_refresh_token,
        user_info.github_refresh_token_expire,
      ) {
        if OffsetDateTime::now_utc() <= access_token_expr {
          // token still valid
          return Some(access_token);
        }

        // we need a new access token
        if OffsetDateTime::now_utc() > refresh_token_expr {
          return None;
        }

        let value = oauth_client
          .exchange_refresh_token(&RefreshToken::new(refresh_token))
          .request_async(async_http_client)
          .await
          .map_err(|err| error!("while trying to perform token exchange {:?}", err))
          .ok()?;

        let access_token = value.access_token().secret();

        let expire_access_token = value.expires_in().unwrap_or(Duration::from_secs(60 * 10));

        if let Err(e) = self
          .update_github_access_token(
            user_id,
            access_token,
            OffsetDateTime::now_utc() + expire_access_token,
          )
          .await
        {
          error!("while trying to update access token {:?}", e);
          return None;
        }

        Some(access_token.clone())
      } else {
        None
      }
    } else {
      None
    }
  }
}
