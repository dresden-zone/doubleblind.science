use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // TODO: CAA, SRV

        db.execute_unprepared(
            "

      CREATE TABLE users (
        id UUID PRIMARY KEY,
        platform INT NOT NULL,
        trusted BOOL NOT NULL,
        admin BOOL NOT NULL,
        github_refresh_token TEXT,
        github_refresh_token_expire TIMESTAMPTZ,
        github_access_token TEXT,
        github_access_token_expire TIMESTAMPTZ,
        github_user_id BIGINT,
        last_update TIMESTAMPTZ NOT NULL
      );

      CREATE TABLE project (
        id UUID PRIMARY KEY,
        owner UUID NOT NULL REFERENCES users(id),
        domain TEXT NOT NULL,
        commit VARCHAR(40) NOT NULL,
        github_id BIGINT,
        created_at TIMESTAMPTZ NOT NULL,
        last_update TIMESTAMPTZ NOT NULL,
        trusted BOOL NOT NULL
      );
    ",
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "
        DROP TABLE github_users;
        DROP TABLE users;
        DROP TABLE record_aaaa;
        DROP TABLE record_cname;
        DROP TABLE record_mx;
        DROP TABLE record_ns;
        DROP TABLE record_txt;
      ",
            )
            .await?;

        Ok(())
    }
}
