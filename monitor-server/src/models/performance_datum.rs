use chrono::{DateTime, FixedOffset};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "performance_data")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub project_id: i32,
    pub app_id: String,
    pub url: Option<String>,
    pub fp: Option<i32>,
    pub fcp: Option<i32>,
    pub lcp: Option<i32>,
    #[sea_orm(column_type = "Decimal(Some((10, 4)))", nullable)]
    pub cls: Option<Decimal>,
    pub ttfb: Option<i32>,
    pub fid: Option<i32>,
    pub load_time: Option<i32>,
    pub dns_time: Option<i32>,
    pub tcp_time: Option<i32>,
    pub ssl_time: Option<i32>,
    pub dom_parse_time: Option<i32>,
    pub resource_count: Option<i32>,
    pub resource_size: Option<i64>,
    pub user_agent: Option<String>,
    pub browser: Option<String>,
    pub device_type: Option<String>,
    pub sdk_version: Option<String>,
    pub release: Option<String>,
    pub environment: Option<String>,
    pub created_at: DateTime<FixedOffset>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
