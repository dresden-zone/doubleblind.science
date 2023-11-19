use entity::{users};
use sea_orm::entity::EntityTrait;
use sea_orm::ActiveValue::Unchanged;
use sea_orm::{ActiveModelBehavior, Related, Set};
use sea_orm::{ActiveModelTrait, DatabaseConnection, Select};
use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

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

    pub(crate) async fn create_github_user(
        &mut self,
        zone_id: Uuid,
        github_token: String,
    ) -> anyhow::Result<users::Model> {
        let user_uuid = Uuid::new_v4();

        Ok(users::ActiveModel {
            id: Set(user_uuid),
            platform: Set(1),
            trusted: Set(false),
            admin: Set(false),
            github_refresh_token: Set(Some(github_token)),
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
                github_refresh_token: Unchanged(user.github_refresh_token),
                last_update: Set(OffsetDateTime::now_utc()),
            }
            .update(&*self.db)
            .await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub(crate) async fn update_github_token(
        &mut self,
        user_id: Uuid,
        token: String,
    ) -> anyhow::Result<bool> {
        let found_user = users::Entity::find_by_id(user_id)
            .one(&*self.db)
            .await?;

        if let Some(user) = found_user {
            users::ActiveModel {
                id: Unchanged(user_id),
                platform: Unchanged(user.platform),
                trusted: Unchanged(user.trusted),
                admin: Unchanged(user.admin),
                github_refresh_token: Set(user.github_refresh_token),
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
