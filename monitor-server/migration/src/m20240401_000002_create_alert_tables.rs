use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // alert_rules
        manager
            .create_table(
                Table::create()
                    .table(AlertRules::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(AlertRules::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(AlertRules::ProjectId).integer().not_null())
                    .col(ColumnDef::new(AlertRules::Name).string_len(200).not_null())
                    .col(
                        ColumnDef::new(AlertRules::RuleType)
                            .string_len(30)
                            .not_null(),
                    )
                    .col(ColumnDef::new(AlertRules::Threshold).integer().null())
                    .col(
                        ColumnDef::new(AlertRules::IntervalMinutes)
                            .integer()
                            .not_null()
                            .default(60),
                    )
                    .col(
                        ColumnDef::new(AlertRules::IsEnabled)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(AlertRules::WebhookUrl)
                            .string_len(500)
                            .null(),
                    )
                    .col(ColumnDef::new(AlertRules::Email).string_len(200).null())
                    .col(
                        ColumnDef::new(AlertRules::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(AlertRules::UpdatedAt)
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
                    .name("idx_alert_rules_project")
                    .table(AlertRules::Table)
                    .col(AlertRules::ProjectId)
                    .to_owned(),
            )
            .await?;

        // alert_logs
        manager
            .create_table(
                Table::create()
                    .table(AlertLogs::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(AlertLogs::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(AlertLogs::RuleId).integer().not_null())
                    .col(ColumnDef::new(AlertLogs::ProjectId).integer().not_null())
                    .col(
                        ColumnDef::new(AlertLogs::AlertType)
                            .string_len(30)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AlertLogs::Severity)
                            .string_len(20)
                            .not_null()
                            .default("warning"),
                    )
                    .col(ColumnDef::new(AlertLogs::Content).text().not_null())
                    .col(ColumnDef::new(AlertLogs::ErrorCount).integer().null())
                    .col(ColumnDef::new(AlertLogs::SampleErrors).json_binary().null())
                    .col(
                        ColumnDef::new(AlertLogs::Status)
                            .string_len(20)
                            .not_null()
                            .default("sent"),
                    )
                    .col(
                        ColumnDef::new(AlertLogs::CreatedAt)
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
                    .name("idx_alert_logs_project_time")
                    .table(AlertLogs::Table)
                    .col(AlertLogs::ProjectId)
                    .col((AlertLogs::CreatedAt, IndexOrder::Desc))
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(AlertLogs::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(AlertRules::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum AlertRules {
    Table,
    Id,
    ProjectId,
    Name,
    RuleType,
    Threshold,
    IntervalMinutes,
    IsEnabled,
    WebhookUrl,
    Email,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum AlertLogs {
    Table,
    Id,
    RuleId,
    ProjectId,
    AlertType,
    Severity,
    Content,
    ErrorCount,
    SampleErrors,
    Status,
    CreatedAt,
}
