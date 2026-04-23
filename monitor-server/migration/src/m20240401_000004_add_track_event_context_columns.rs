//! 补充埋点事件上下文字段。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(TrackEvents::Table)
                    .add_column(ColumnDef::new(TrackEvents::Viewport).string_len(30).null())
                    .add_column(
                        ColumnDef::new(TrackEvents::ScreenResolution)
                            .string_len(30)
                            .null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(TrackEvents::Table)
                    .drop_column(TrackEvents::ScreenResolution)
                    .drop_column(TrackEvents::Viewport)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum TrackEvents {
    Table,
    Viewport,
    ScreenResolution,
}
