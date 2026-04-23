use chrono::{DateTime, FixedOffset};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "ai_analyses")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub error_id: i64,
    pub fingerprint: Option<String>,
    pub project_id: i32,
    pub model_used: Option<String>,
    pub prompt_tokens: Option<i32>,
    pub completion_tokens: Option<i32>,
    pub cost_ms: Option<i32>,
    pub status: String,
    #[sea_orm(column_type = "Text", nullable)]
    pub ai_suggestion: Option<String>,
    pub severity_score: Option<i16>,
    pub confidence: Option<f32>,
    pub probable_file: Option<String>,
    pub probable_line: Option<i32>,
    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub tags: Option<Json>,
    #[sea_orm(column_type = "Text", nullable)]
    pub analyzed_stack: Option<String>,
    pub is_cached: bool,
    pub cache_key: Option<String>,
    pub created_at: DateTime<FixedOffset>,
    pub updated_at: DateTime<FixedOffset>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
