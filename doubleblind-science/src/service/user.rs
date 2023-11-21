use std::sync::Arc;

use sea_orm::entity::EntityTrait;
use sea_orm::ActiveValue::Unchanged;
use sea_orm::ColumnTrait;
use sea_orm::QueryFilter;
use sea_orm::Set;
use sea_orm::{ActiveModelTrait, DatabaseConnection};
use time::OffsetDateTime;
use uuid::Uuid;

use entity::users;

#[derive(Clone)]
pub(crate) struct UserService {
    db: Arc<DatabaseConnection>,
}

impl UserService {
    pub(crate) fn from_db(db: Arc<DatabaseConnection>) -> UserService {
        UserService { db }
    }

    pub(crate) async fn all_users(&mut self, zone_id: Uuid) -> anyhow::Result<Vec<users::Model>> {
        Ok(users::Entity::find().all(&*self.db).await?)
    }
    pub(crate) async fn get_user(&mut self, user_id: Uuid) -> anyhow::Result<Option<users::Model>> {
        Ok(users::Entity::find_by_id(user_id).one(&*self.db).await?)
    }

    pub(crate) async fn get_user_by_github(
        &mut self,
        github_id: i64,
    ) -> anyhow::Result<Option<users::Model>> {
        Ok(users::Entity::find()
            .filter(users::Column::GithubUserId.eq(github_id))
            .one(&*self.db)
            .await?)
    }

    pub(crate) async fn create_github_user(
        &mut self,
        github_id: i64,
        github_refresh_token: String,
        github_refresh_token_expire: OffsetDateTime,
        github_access_token: String,
        github_access_token_expire: OffsetDateTime,
    ) -> anyhow::Result<users::Model> {
        let user_uuid = Uuid::new_v4();

        Ok(users::ActiveModel {
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
        .await?)
    }

    pub(crate) async fn delete(&mut self, user_id: Uuid) -> anyhow::Result<bool> {
        Ok(users::Entity::delete_by_id(user_id)
            .exec(&*self.db)
            .await?
            .rows_affected
            > 0)
    }

    pub(crate) async fn update(
        &mut self,
        user_id: Uuid,
        trusted: bool,
        admin: bool,
    ) -> anyhow::Result<bool> {
        let found_user = users::Entity::find_by_id(user_id).one(&*self.db).await?;

        if let Some(user) = found_user {
            users::ActiveModel {
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
        github_access_token: String,
        github_access_token_expire: OffsetDateTime,
    ) -> anyhow::Result<bool> {
        let found_user = users::Entity::find_by_id(user_id).one(&*self.db).await?;

        if let Some(user) = found_user {
            users::ActiveModel {
                id: Unchanged(user_id),
                platform: Unchanged(user.platform),
                trusted: Unchanged(user.trusted),
                admin: Unchanged(user.admin),
                github_user_id: Unchanged(user.github_user_id),
                github_refresh_token: Set(user.github_refresh_token),
                github_refresh_token_expire: Set(user.github_refresh_token_expire),
                github_access_token: Set(Some(github_access_token)),
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
}
