use chrono::{DateTime, FixedOffset};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "js_errors")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub project_id: i32,
    pub app_id: String,
    #[sea_orm(column_type = "Text")]
    pub message: String,
    #[sea_orm(column_type = "Text", nullable)]
    pub stack: Option<String>,
    pub error_type: String,
    pub source_url: Option<String>,
    pub line: Option<i32>,
    pub column: Option<i32>,
    pub user_agent: Option<String>,
    pub browser: Option<String>,
    pub browser_version: Option<String>,
    pub os: Option<String>,
    pub os_version: Option<String>,
    pub device: Option<String>,
    pub device_type: Option<String>,
    pub url: Option<String>,
    pub referrer: Option<String>,
    pub viewport: Option<String>,
    pub screen_resolution: Option<String>,
    pub language: Option<String>,
    pub timezone: Option<String>,
    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub breadcrumb: Option<Json>,
    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub extra: Option<Json>,
    pub fingerprint: Option<String>,
    pub sdk_version: Option<String>,
    pub release: Option<String>,
    pub environment: Option<String>,
    pub is_ai_analyzed: bool,
    pub distinct_id: Option<String>,
    pub created_at: DateTime<FixedOffset>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
