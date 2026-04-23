use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(AiAnalyses::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(AiAnalyses::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(AiAnalyses::ErrorId).big_integer().not_null())
                    .col(
                        ColumnDef::new(AiAnalyses::Fingerprint)
                            .string_len(128)
                            .null(),
                    )
                    .col(ColumnDef::new(AiAnalyses::ProjectId).integer().not_null())
                    .col(ColumnDef::new(AiAnalyses::ModelUsed).string_len(64).null())
                    .col(ColumnDef::new(AiAnalyses::PromptTokens).integer().null())
                    .col(
                        ColumnDef::new(AiAnalyses::CompletionTokens)
                            .integer()
                            .null(),
                    )
                    .col(ColumnDef::new(AiAnalyses::CostMs).integer().null())
                    .col(
                        ColumnDef::new(AiAnalyses::Status)
                            .string_len(20)
                            .not_null()
                            .default("pending"),
                    )
                    .col(ColumnDef::new(AiAnalyses::AiSuggestion).text().null())
                    .col(
                        ColumnDef::new(AiAnalyses::SeverityScore)
                            .small_integer()
                            .null(),
                    )
                    .col(ColumnDef::new(AiAnalyses::Confidence).float().null())
                    .col(
                        ColumnDef::new(AiAnalyses::ProbableFile)
                            .string_len(500)
                            .null(),
                    )
                    .col(ColumnDef::new(AiAnalyses::ProbableLine).integer().null())
                    .col(ColumnDef::new(AiAnalyses::Tags).json_binary().null())
                    .col(ColumnDef::new(AiAnalyses::AnalyzedStack).text().null())
                    .col(
                        ColumnDef::new(AiAnalyses::IsCached)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(ColumnDef::new(AiAnalyses::CacheKey).string_len(64).null())
                    .col(
                        ColumnDef::new(AiAnalyses::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(AiAnalyses::UpdatedAt)
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
                    .name("idx_ai_analyses_error_id")
                    .table(AiAnalyses::Table)
                    .col(AiAnalyses::ErrorId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_ai_analyses_fingerprint")
                    .table(AiAnalyses::Table)
                    .col(AiAnalyses::Fingerprint)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(AiAnalyses::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum AiAnalyses {
    Table,
    Id,
    ErrorId,
    Fingerprint,
    ProjectId,
    ModelUsed,
    PromptTokens,
    CompletionTokens,
    CostMs,
    Status,
    AiSuggestion,
    SeverityScore,
    Confidence,
    ProbableFile,
    ProbableLine,
    Tags,
    AnalyzedStack,
    IsCached,
    CacheKey,
    CreatedAt,
    UpdatedAt,
}
