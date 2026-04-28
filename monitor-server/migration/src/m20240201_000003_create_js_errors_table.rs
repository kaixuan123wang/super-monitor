//! JS 错误主表。
//!
//! 注：设计文档提到按月分区，Phase 2 先用普通表 + 索引满足中小规模需求，
//! 后续可通过 `ALTER TABLE ... PARTITION` 或迁移方案升级为分区表。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(JsErrors::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(JsErrors::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(JsErrors::ProjectId).integer().not_null())
                    .col(ColumnDef::new(JsErrors::AppId).string_len(32).not_null())
                    .col(ColumnDef::new(JsErrors::Message).text().not_null())
                    .col(ColumnDef::new(JsErrors::Stack).text().null())
                    .col(
                        ColumnDef::new(JsErrors::ErrorType)
                            .string_len(50)
                            .not_null(),
                    )
                    .col(ColumnDef::new(JsErrors::SourceUrl).string_len(500).null())
                    .col(ColumnDef::new(JsErrors::Line).integer().null())
                    .col(ColumnDef::new(JsErrors::Column).integer().null())
                    .col(ColumnDef::new(JsErrors::UserAgent).string_len(500).null())
                    .col(ColumnDef::new(JsErrors::Browser).string_len(50).null())
                    .col(
                        ColumnDef::new(JsErrors::BrowserVersion)
                            .string_len(30)
                            .null(),
                    )
                    .col(ColumnDef::new(JsErrors::Os).string_len(50).null())
                    .col(ColumnDef::new(JsErrors::OsVersion).string_len(30).null())
                    .col(ColumnDef::new(JsErrors::Device).string_len(50).null())
                    .col(ColumnDef::new(JsErrors::DeviceType).string_len(20).null())
                    .col(ColumnDef::new(JsErrors::Url).string_len(500).null())
                    .col(ColumnDef::new(JsErrors::Referrer).string_len(500).null())
                    .col(ColumnDef::new(JsErrors::Viewport).string_len(30).null())
                    .col(
                        ColumnDef::new(JsErrors::ScreenResolution)
                            .string_len(30)
                            .null(),
                    )
                    .col(ColumnDef::new(JsErrors::Language).string_len(10).null())
                    .col(ColumnDef::new(JsErrors::Timezone).string_len(50).null())
                    .col(ColumnDef::new(JsErrors::Breadcrumb).json_binary().null())
                    .col(ColumnDef::new(JsErrors::Extra).json_binary().null())
                    .col(ColumnDef::new(JsErrors::Fingerprint).string_len(64).null())
                    .col(ColumnDef::new(JsErrors::SdkVersion).string_len(20).null())
                    .col(ColumnDef::new(JsErrors::Release).string_len(50).null())
                    .col(ColumnDef::new(JsErrors::Environment).string_len(20).null())
                    .col(
                        ColumnDef::new(JsErrors::IsAiAnalyzed)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(ColumnDef::new(JsErrors::DistinctId).string_len(128).null())
                    .col(
                        ColumnDef::new(JsErrors::CreatedAt)
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
                    .name("idx_js_errors_project_time")
                    .table(JsErrors::Table)
                    .col(JsErrors::ProjectId)
                    .col((JsErrors::CreatedAt, IndexOrder::Desc))
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_js_errors_fingerprint")
                    .table(JsErrors::Table)
                    .col(JsErrors::Fingerprint)
                    .col((JsErrors::CreatedAt, IndexOrder::Desc))
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_js_errors_type")
                    .table(JsErrors::Table)
                    .col(JsErrors::ErrorType)
                    .col((JsErrors::CreatedAt, IndexOrder::Desc))
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(JsErrors::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum JsErrors {
    Table,
    Id,
    ProjectId,
    AppId,
    Message,
    Stack,
    ErrorType,
    SourceUrl,
    Line,
    Column,
    UserAgent,
    Browser,
    BrowserVersion,
    Os,
    OsVersion,
    Device,
    DeviceType,
    Url,
    Referrer,
    Viewport,
    ScreenResolution,
    Language,
    Timezone,
    Breadcrumb,
    Extra,
    Fingerprint,
    SdkVersion,
    Release,
    Environment,
    IsAiAnalyzed,
    DistinctId,
    CreatedAt,
}
