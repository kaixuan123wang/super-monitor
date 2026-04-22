//! 用户自定义埋点事件定义表（Phase 3）。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(TrackEventDefinitions::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(TrackEventDefinitions::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(TrackEventDefinitions::ProjectId).integer().not_null())
                    .col(ColumnDef::new(TrackEventDefinitions::EventName).string_len(128).not_null())
                    .col(ColumnDef::new(TrackEventDefinitions::DisplayName).string_len(128).null())
                    .col(ColumnDef::new(TrackEventDefinitions::Category).string_len(50).null())
                    .col(ColumnDef::new(TrackEventDefinitions::Description).text().null())
                    // properties: [{name, type, description, required}]
                    .col(ColumnDef::new(TrackEventDefinitions::Properties).json_binary().null())
                    .col(
                        ColumnDef::new(TrackEventDefinitions::Status)
                            .string_len(20)
                            .not_null()
                            .default("active"),
                    )
                    .col(
                        ColumnDef::new(TrackEventDefinitions::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(TrackEventDefinitions::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_event_def_unique")
                    .table(TrackEventDefinitions::Table)
                    .col(TrackEventDefinitions::ProjectId)
                    .col(TrackEventDefinitions::EventName)
                    .unique()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(TrackEventDefinitions::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum TrackEventDefinitions {
    Table,
    Id,
    ProjectId,
    EventName,
    DisplayName,
    Category,
    Description,
    Properties,
    Status,
    CreatedAt,
    UpdatedAt,
}
