use std::sync::Arc;

use entity::{project, user};
use sea_orm::entity::EntityTrait;
use sea_orm::ActiveValue::Unchanged;
use sea_orm::ColumnTrait;
use sea_orm::QueryFilter;
use sea_orm::Set;
use sea_orm::{ActiveModelTrait, DatabaseConnection};
use sea_query::Condition;
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

  pub(crate) async fn all_projects(&self, _zone_id: Uuid) -> anyhow::Result<Vec<project::Model>> {
    Ok(project::Entity::find().all(&*self.db).await?)
  }
  pub(crate) async fn get_project(
    &self,
    project_id: Uuid,
  ) -> anyhow::Result<Option<project::Model>> {
    Ok(
      project::Entity::find_by_id(project_id)
        .one(&*self.db)
        .await?,
    )
  }

  pub(crate) async fn create_project(
    &self,
    owner_uuid: Uuid,
    domain: String,
    commit: String,
    github_id: i64,
    secret_token: String,
  ) -> anyhow::Result<Option<project::Model>> {
    let project_uuid = Uuid::new_v4();

    if let Some(user) = user::Entity::find_by_id(owner_uuid).one(&*self.db).await? {
      Ok(Some(
        project::ActiveModel {
          id: Set(project_uuid),
          owner: Set(owner_uuid),
          domain: Set(domain),
          commit: Set(commit),
          github_id: Set(Some(github_id)),
          created_at: Set(OffsetDateTime::now_utc()),
          last_update: Set(OffsetDateTime::now_utc()),
          github_webhook_secret: Set(Some(secret_token)),
          trusted: Set(user.trusted),
        }
        .insert(&*self.db)
        .await?,
      ))
    } else {
      Ok(None)
    }
  }

  pub(crate) async fn delete(&self, project_id: Uuid) -> anyhow::Result<bool> {
    Ok(
      project::Entity::delete_by_id(project_id)
        .exec(&*self.db)
        .await?
        .rows_affected
        > 0,
    )
  }

  pub(crate) async fn trust_project(&self, project_id: Uuid) -> anyhow::Result<bool> {
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
        github_webhook_secret: Unchanged(project.github_webhook_secret),
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

  pub(crate) async fn get_user_projects(
    &self,
    user_id: Uuid,
  ) -> anyhow::Result<Vec<project::Model>> {
    Ok(
      project::Entity::find()
        .filter(project::Column::Owner.eq(user_id))
        .all(&*self.db)
        .await?,
    )
  }

  pub(crate) async fn get_projects_by_github_id(
    &self,
    github_id: i64,
  ) -> anyhow::Result<Option<project::Model>> {
    Ok(
      project::Entity::find()
        .filter(project::Column::GithubId.eq(github_id))
        .one(&*self.db)
        .await?,
    )
  }

  pub(crate) async fn get_project_by_name_or_repo(
    &self,
    name: &str,
    repo: i64,
  ) -> anyhow::Result<Option<project::Model>> {
    Ok(
      project::Entity::find()
        .filter(
          Condition::any()
            .add(project::Column::Domain.eq(name))
            .add(project::Column::GithubId.eq(repo)),
        )
        .one(&*self.db)
        .await?,
    )
  }
}
