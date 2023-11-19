use entity::{project, users};
use sea_orm::entity::EntityTrait;
use sea_orm::ActiveValue::Unchanged;
use sea_orm::{ActiveModelBehavior, Related, Set};
use sea_orm::{ActiveModelTrait, DatabaseConnection, Select};
use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Clone)]
pub(crate) struct ProjectService {
    db: Arc<DatabaseConnection>,
}

impl ProjectService {
    pub(crate) fn from_db(db: Arc<DatabaseConnection>) -> ProjectService {
        ProjectService { db }
    }

    pub(crate) async fn all_projects(
        &mut self,
        zone_id: Uuid,
    ) -> anyhow::Result<Vec<project::Model>> {
        Ok(project::Entity::find().all(&*self.db).await?)
    }
    pub(crate) async fn get_project(
        &mut self,
        project_id: Uuid,
    ) -> anyhow::Result<Option<project::Model>> {
        Ok(project::Entity::find_by_id(project_id)
            .one(&*self.db)
            .await?)
    }

    pub(crate) async fn create_project(
        &mut self,
        owner_uuid: Uuid,
        domain: String,
        commit: String,
        github_id: i64,
    ) -> anyhow::Result<Option<project::Model>> {
        let project_uuid = Uuid::new_v4();

        if let Some(user) = users::Entity::find_by_id(owner_uuid).one(&*self.db).await? {
            Ok(Some(project::ActiveModel {
                id: Set(project_uuid),
                owner: Set(owner_uuid),
                domain: Set(domain),
                commit: Set(commit),
                github_id: Set(Some(github_id)),
                created_at: Set(OffsetDateTime::now_utc()),
                last_update: Set(OffsetDateTime::now_utc()),
                trusted: Set(user.trusted),
            }
            .insert(&*self.db)
            .await?))
        } else {
            Ok(None)
        }
    }

    pub(crate) async fn delete(&mut self, project_id: Uuid) -> anyhow::Result<bool> {
        Ok(project::Entity::delete_by_id(project_id)
            .exec(&*self.db)
            .await?
            .rows_affected
            > 0)
    }

    pub(crate) async fn trust_project(&mut self, project_id: Uuid) -> anyhow::Result<bool> {
        let found_project = project::Entity::find_by_id(project_id)
            .one(&*self.db)
            .await?;

        if let Some(project) = found_project {
            project::ActiveModel {
                id: Unchanged(project_id),
                owner: Unchanged(project.owner),
                domain: Unchanged(project.domain),
                commit: Unchanged(project.commit),
                created_at: Unchanged(project.created_at),
                github_id: Unchanged(project.github_id),
                last_update: Set(OffsetDateTime::now_utc()),
                trusted: Set(!project.trusted),
            }
            .update(&*self.db)
            .await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
    /*
    pub(crate) async fn update_github_token(
      &mut self,
      user_id: Uuid,
      token: String
    ) -> anyhow::Result<bool>  {
      let found_user = github_users::Entity::find_by_id(user_id).one(&*self.db).await?;

      if let Some(user) = found_user {
        github_users::ActiveModel {
          id: Unchanged(user_id),
          refresh_token: Set(token),
          last_update: Set(OffsetDateTime::now_utc())
        }.update(&*self.db).await?;
        Ok(true)
      } else {
        Ok(false)
      }
    }*/
}
