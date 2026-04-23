use chrono::{DateTime, FixedOffset};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "track_retention_configs")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub project_id: i32,
    pub name: String,
    pub initial_event: String,
    pub return_event: String,
    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub initial_filters: Option<Json>,
    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub return_filters: Option<Json>,
    pub retention_days: i32,
    pub created_by: Option<i32>,
    pub created_at: DateTime<FixedOffset>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
