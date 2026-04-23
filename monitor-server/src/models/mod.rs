//! SeaORM Entity 模块。
//!
//! 手动维护（对应 `migration/` 下的各表）。

pub mod ai_analysis;
pub mod alert_log;
pub mod alert_rule;
pub mod event_definition;
pub mod group;
pub mod js_error;
pub mod network_error;
pub mod performance_datum;
pub mod project;
pub mod project_member;
pub mod source_map;
pub mod track_event;
pub mod track_event_stats;
pub mod track_funnel;
pub mod track_id_mapping;
pub mod track_retention_config;
pub mod track_user_profile;
pub mod user;

pub use ai_analysis::Entity as AiAnalysis;
pub use alert_log::Entity as AlertLog;
pub use alert_rule::Entity as AlertRule;
pub use event_definition::Entity as EventDefinition;
pub use group::Entity as Group;
pub use js_error::Entity as JsError;
pub use network_error::Entity as NetworkError;
pub use performance_datum::Entity as PerformanceDatum;
pub use project::Entity as Project;
pub use project_member::Entity as ProjectMember;
pub use source_map::Entity as SourceMap;
pub use track_event::Entity as TrackEvent;
pub use track_event_stats::Entity as TrackEventStats;
pub use track_funnel::Entity as TrackFunnel;
pub use track_id_mapping::Entity as TrackIdMapping;
pub use track_retention_config::Entity as TrackRetentionConfig;
pub use track_user_profile::Entity as TrackUserProfile;
pub use user::Entity as User;
