use chrono::{DateTime, FixedOffset};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "network_errors")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub project_id: i32,
    pub app_id: String,
    pub url: String,
    pub method: String,
    pub status: Option<i32>,
    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub request_headers: Option<Json>,
    #[sea_orm(column_type = "Text", nullable)]
    pub request_body: Option<String>,
    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub response_headers: Option<Json>,
    #[sea_orm(column_type = "Text", nullable)]
    pub response_text: Option<String>,
    pub duration: Option<i32>,
    pub error_type: Option<String>,
    pub user_agent: Option<String>,
    pub browser: Option<String>,
    pub os: Option<String>,
    pub device: Option<String>,
    pub sdk_version: Option<String>,
    pub release: Option<String>,
    pub environment: Option<String>,
    pub page_url: Option<String>,
    pub distinct_id: Option<String>,
    pub created_at: DateTime<FixedOffset>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
