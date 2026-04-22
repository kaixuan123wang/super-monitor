use chrono::{DateTime, FixedOffset};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "track_events")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub project_id: i32,
    pub app_id: String,
    pub distinct_id: String,
    pub anonymous_id: Option<String>,
    pub user_id: Option<String>,
    pub is_login_id: bool,
    pub event: String,
    pub event_type: String,
    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub properties: Option<Json>,
    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub super_properties: Option<Json>,
    pub session_id: Option<String>,
    #[sea_orm(column_type = "Decimal(Some((10, 3)))", nullable)]
    pub event_duration: Option<Decimal>,
    pub page_url: Option<String>,
    pub page_title: Option<String>,
    pub referrer: Option<String>,
    pub user_agent: Option<String>,
    pub browser: Option<String>,
    pub browser_version: Option<String>,
    pub os: Option<String>,
    pub os_version: Option<String>,
    pub device_type: Option<String>,
    pub language: Option<String>,
    pub timezone: Option<String>,
    pub sdk_version: Option<String>,
    pub release: Option<String>,
    pub environment: Option<String>,
    pub client_time: Option<DateTime<FixedOffset>>,
    pub created_at: DateTime<FixedOffset>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
