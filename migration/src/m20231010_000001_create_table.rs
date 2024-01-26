use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    let db = manager.get_connection();

    // TODO: CAA, SRV

    db.execute_unprepared(
      r#"
      CREATE TABLE github_app (
        id UUID PRIMARY KEY,
        installation_id BIGINT NOT NULL,
        github_access_token TEXT NOT NULL,
        github_access_token_expire TIMESTAMPTZ NOT NULL,
        last_update TIMESTAMPTZ NOT NULL
      );

      CREATE TABLE repository (
        id UUID PRIMARY KEY,
        github_app UUID NOT NULL REFERENCES github_app(id),
        domain TEXT,
        github_full_name TEXT NOT NULL,
        github_short_name TEXT NOT NULL,
        github_id BIGINT NOT NULL,
        trusted BOOL NOT NULL,
        deployed BOOL NOT NULL,
        last_update TIMESTAMPTZ NOT NULL,
        created_at TIMESTAMPTZ NOT NULL
      );
    "#,
    )
    .await?;

    Ok(())
  }

  async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager
      .get_connection()
      .execute_unprepared(
        r#"
        DROP TABLE github_users;
        DROP TABLE "user";
        DROP TABLE record_aaaa;
        DROP TABLE record_cname;
        DROP TABLE record_mx;
        DROP TABLE record_ns;
        DROP TABLE record_txt;
      "#,
      )
      .await?;

    Ok(())
  }
}
