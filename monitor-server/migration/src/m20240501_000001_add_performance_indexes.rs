//! 添加性能优化索引。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // js_errors 表索引
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_js_errors_project_created")
                    .table(JsError::Table)
                    .col(JsError::ProjectId)
                    .col(JsError::CreatedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_js_errors_fingerprint")
                    .table(JsError::Table)
                    .col(JsError::Fingerprint)
                    .to_owned(),
            )
            .await?;

        // network_errors 表索引
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_network_errors_project_created")
                    .table(NetworkError::Table)
                    .col(NetworkError::ProjectId)
                    .col(NetworkError::CreatedAt)
                    .to_owned(),
            )
            .await?;

        // performance_data 表索引
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_performance_data_project_created")
                    .table(PerformanceDatum::Table)
                    .col(PerformanceDatum::ProjectId)
                    .col(PerformanceDatum::CreatedAt)
                    .to_owned(),
            )
            .await?;

        // track_events 表索引
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_track_events_project_event_created")
                    .table(TrackEvent::Table)
                    .col(TrackEvent::ProjectId)
                    .col(TrackEvent::Event)
                    .col(TrackEvent::CreatedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_track_events_project_distinct")
                    .table(TrackEvent::Table)
                    .col(TrackEvent::ProjectId)
                    .col(TrackEvent::DistinctId)
                    .to_owned(),
            )
            .await?;

        // track_event_stats 表索引
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_track_event_stats_project_event_hour")
                    .table(TrackEventStats::Table)
                    .col(TrackEventStats::ProjectId)
                    .col(TrackEventStats::Event)
                    .col(TrackEventStats::Hour)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // ai_analyses 表索引
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_ai_analyses_error_id")
                    .table(AiAnalysis::Table)
                    .col(AiAnalysis::ErrorId)
                    .to_owned(),
            )
            .await?;

        // alert_rules 表索引
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_alert_rules_project_enabled")
                    .table(AlertRule::Table)
                    .col(AlertRule::ProjectId)
                    .col(AlertRule::Enabled)
                    .to_owned(),
            )
            .await?;

        // alert_logs 表索引
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_alert_logs_rule_id_created")
                    .table(AlertLog::Table)
                    .col(AlertLog::RuleId)
                    .col(AlertLog::CreatedAt)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("idx_js_errors_project_created")
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(Index::drop().name("idx_js_errors_fingerprint").to_owned())
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_network_errors_project_created")
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_performance_data_project_created")
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_track_events_project_event_created")
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_track_events_project_distinct")
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_track_event_stats_project_event_hour")
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(Index::drop().name("idx_ai_analyses_error_id").to_owned())
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_alert_rules_project_enabled")
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_alert_logs_rule_id_created")
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(Iden)]
enum JsError {
    Table,
    ProjectId,
    CreatedAt,
    Fingerprint,
}

#[derive(Iden)]
enum NetworkError {
    Table,
    ProjectId,
    CreatedAt,
}

#[derive(Iden)]
enum PerformanceDatum {
    Table,
    ProjectId,
    CreatedAt,
}

#[derive(Iden)]
enum TrackEvent {
    Table,
    ProjectId,
    Event,
    CreatedAt,
    DistinctId,
}

#[derive(Iden)]
enum TrackEventStats {
    Table,
    ProjectId,
    Event,
    Hour,
}

#[derive(Iden)]
enum AiAnalysis {
    Table,
    ErrorId,
}

#[derive(Iden)]
enum AlertRule {
    Table,
    ProjectId,
    Enabled,
}

#[derive(Iden)]
enum AlertLog {
    Table,
    RuleId,
    CreatedAt,
}
