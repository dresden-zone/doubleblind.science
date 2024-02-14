use std::sync::Arc;

use sea_orm::entity::EntityTrait;
use sea_orm::ActiveValue::Unchanged;
use sea_orm::QueryFilter;
use sea_orm::Set;
use sea_orm::{ActiveModelTrait, DatabaseConnection};
use sea_orm::{ColumnTrait, NotSet};
use sea_query::Expr;
use time::OffsetDateTime;
use uuid::Uuid;

use entity::github_app::Model;
use entity::prelude::Repository;
use entity::{github_app, repository};

use crate::routes::RepoInformation;

#[derive(Clone)]
pub(crate) struct ProjectService {
  db: Arc<DatabaseConnection>,
}

impl ProjectService {
  pub(crate) fn from_db(db: Arc<DatabaseConnection>) -> ProjectService {
    ProjectService { db }
  }

  pub(crate) async fn all_github_app_installations(&self) -> anyhow::Result<Vec<Model>> {
    Ok(github_app::Entity::find().all(&*self.db).await?)
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

  pub(crate) async fn create_github_app(
    &self,
    installation_id: i64,
    access_token: &String,
    access_token_expire: OffsetDateTime,
  ) -> anyhow::Result<Model> {
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
          github_access_token: Set(access_token.clone()),
          github_access_token_expire: Set(access_token_expire),
          last_update: Set(OffsetDateTime::now_utc()),
        }
        .insert(&*self.db)
        .await?,
      ),
    }
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

  pub(crate) async fn get_repository(&self, id: i64) -> anyhow::Result<Option<repository::Model>> {
    Ok(
      repository::Entity::find()
        .filter(repository::Column::GithubId.eq(id))
        .one(&*self.db)
        .await?,
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
        github_access_token: Set(access_token.clone()),
        github_access_token_expire: Set(access_token_expire),
        last_update: Set(OffsetDateTime::now_utc()),
      }
      .update(&*self.db)
      .await?,
    ))
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
    names: Vec<RepoInformation>,
  ) -> anyhow::Result<()> {
    repository::Entity::delete_many()
      .filter(repository::Column::GithubApp.eq(app_id))
      .exec(&*self.db)
      .await?;

    Repository::insert_many(names.into_iter().map(|info| repository::ActiveModel {
      id: Set(Uuid::new_v4()),
      github_app: Set(app_id),
      domain: NotSet,
      github_id: Set(info.id),
      github_short_name: Set(info.short_name),
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
  ) -> anyhow::Result<Vec<repository::Model>> {
    Ok(
      Repository::update_many()
        .col_expr(repository::Column::Deployed, Expr::value(true))
        .col_expr(repository::Column::Domain, Expr::value(domain))
        .filter(repository::Column::GithubId.eq(github_id))
        .exec_with_returning(&*self.db)
        .await?,
    )
  }
}
