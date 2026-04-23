use chrono::{DateTime, FixedOffset};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "alert_logs")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub rule_id: i32,
    pub project_id: i32,
    pub alert_type: String,
    pub severity: String,
    #[sea_orm(column_type = "Text")]
    pub content: String,
    pub error_count: Option<i32>,
    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub sample_errors: Option<Json>,
    pub status: String,
    pub created_at: DateTime<FixedOffset>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
