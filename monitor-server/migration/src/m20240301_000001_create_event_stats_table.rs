//! 埋点事件按小时预聚合表（Phase 3）。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(TrackEventStatsHourly::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(TrackEventStatsHourly::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(TrackEventStatsHourly::ProjectId)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(TrackEventStatsHourly::Event)
                            .string_len(128)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(TrackEventStatsHourly::Hour)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(TrackEventStatsHourly::TotalCount)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(TrackEventStatsHourly::UniqueUsers)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(TrackEventStatsHourly::PropertiesSummary)
                            .json_binary()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_track_stats_unique")
                    .table(TrackEventStatsHourly::Table)
                    .col(TrackEventStatsHourly::ProjectId)
                    .col(TrackEventStatsHourly::Event)
                    .col(TrackEventStatsHourly::Hour)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_track_stats_project_hour")
                    .table(TrackEventStatsHourly::Table)
                    .col(TrackEventStatsHourly::ProjectId)
                    .col((TrackEventStatsHourly::Hour, IndexOrder::Desc))
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(TrackEventStatsHourly::Table)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
pub enum TrackEventStatsHourly {
    Table,
    Id,
    ProjectId,
    Event,
    Hour,
    TotalCount,
    UniqueUsers,
    PropertiesSummary,
}
