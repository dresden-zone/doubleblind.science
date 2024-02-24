use std::sync::Arc;

use sea_orm::entity::EntityTrait;
use sea_orm::QueryFilter;
use sea_orm::Set;
use sea_orm::{ActiveModelTrait, DatabaseConnection};
use sea_orm::{ColumnTrait, NotSet};
use sea_query::Expr;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::routes::{GithubRepoEdit, GithubRepoInformation};
use entity::github_app::Model;
use entity::prelude::Repository;
use entity::{github_app, repository};

#[derive(Clone)]
pub(crate) struct ProjectService {
  db: Arc<DatabaseConnection>,
}

impl ProjectService {
  pub(crate) fn from_db(db: Arc<DatabaseConnection>) -> ProjectService {
    ProjectService { db }
  }

  pub(crate) async fn get_github_app(&self, installation_id: i64) -> anyhow::Result<Option<Model>> {
    Ok(
      github_app::Entity::find()
        .filter(github_app::Column::InstallationId.eq(installation_id))
        .one(&*self.db)
        .await?,
    )
  }

  pub(crate) async fn get_github_app_uuid(&self, id: Uuid) -> anyhow::Result<Option<Model>> {
    Ok(github_app::Entity::find_by_id(id).one(&*self.db).await?)
  }

  pub(crate) async fn create_github_app(&self, installation_id: i64) -> anyhow::Result<Model> {
    match github_app::Entity::find()
      .filter(github_app::Column::InstallationId.eq(installation_id))
      .one(&*self.db)
      .await?
    {
      Some(value) => Ok(value),
      None => Ok(
        github_app::ActiveModel {
          id: Set(Uuid::new_v4()),
          installation_id: Set(installation_id),
          last_update: Set(OffsetDateTime::now_utc()),
        }
        .insert(&*self.db)
        .await?,
      ),
    }
  }

  pub(crate) async fn get_repository(&self, id: i64) -> anyhow::Result<Option<repository::Model>> {
    Ok(
      repository::Entity::find()
        .filter(repository::Column::GithubId.eq(id))
        .one(&*self.db)
        .await?,
    )
  }

  pub(crate) async fn all_repos_for_installation_id(
    &self,
    installation_id: i64,
  ) -> anyhow::Result<Option<Vec<repository::Model>>> {
    let found_github_app: Model = match github_app::Entity::find()
      .filter(github_app::Column::InstallationId.eq(installation_id))
      .one(&*self.db)
      .await?
    {
      Some(value) => value,
      None => {
        return Ok(None);
      }
    };

    Ok(Some(
      repository::Entity::find()
        .filter(repository::Column::GithubApp.eq(found_github_app.id))
        .all(&*self.db)
        .await?,
    ))
  }

  pub(crate) async fn rewrite_list_of_repositories(
    &self,
    app_id: Uuid,
    names: Vec<GithubRepoEdit>,
  ) -> anyhow::Result<()> {
    repository::Entity::delete_many()
      .filter(repository::Column::GithubApp.eq(app_id))
      .exec(&*self.db)
      .await?;

    Repository::insert_many(names.into_iter().map(|info| repository::ActiveModel {
      id: Set(Uuid::new_v4()),
      github_app: Set(app_id),
      domain: NotSet,
      branch: NotSet,
      github_id: Set(info.id),
      github_short_name: Set(info.name),
      github_full_name: Set(info.full_name),
      trusted: Set(false),
      deployed: Set(false),
      created_at: Set(OffsetDateTime::now_utc()),
      last_update: Set(OffsetDateTime::now_utc()),
    }))
    .exec(&*self.db)
    .await?;

    Ok(())
  }

  pub(crate) async fn deploy_repo(
    &self,
    github_id: i64,
    domain: String,
    branch: String,
  ) -> anyhow::Result<Vec<repository::Model>> {
    Ok(
      Repository::update_many()
        .col_expr(repository::Column::Deployed, Expr::value(true))
        .col_expr(repository::Column::Domain, Expr::value(domain))
        .col_expr(repository::Column::Branch, Expr::value(branch))
        .filter(repository::Column::GithubId.eq(github_id))
        .exec_with_returning(&*self.db)
        .await?,
    )
  }
}
