//! `SeaORM` Entity for semester_summary table

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Copy, Clone, Default, Debug, DeriveEntity)]
pub struct Entity;

impl EntityName for Entity {
    fn table_name(&self) -> &str {
        "semester_summary"
    }
}

#[derive(Clone, Debug, PartialEq, DeriveModel, DeriveActiveModel, Eq, Serialize, Deserialize)]
pub struct Model {
    #[serde(skip_deserializing)]
    pub summary_id: Uuid,
    pub user_id: Uuid,
    pub semester: String,
    pub academic_year: String,
    pub gpa: Decimal,
    pub classification: Option<String>,
    pub total_credits: Option<i32>,
    pub total_passed_credits: Option<i32>,
    pub metadata: Option<Value>,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveColumn)]
pub enum Column {
    SummaryId,
    UserId,
    Semester,
    AcademicYear,
    Gpa,
    Classification,
    TotalCredits,
    TotalPassedCredits,
    Metadata,
    CreatedAt,
    UpdatedAt,
}

#[derive(Copy, Clone, Debug, EnumIter, DerivePrimaryKey)]
pub enum PrimaryKey {
    SummaryId,
}

impl PrimaryKeyTrait for PrimaryKey {
    type ValueType = Uuid;
    fn auto_increment() -> bool {
        false
    }
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    User,
}

impl ColumnTrait for Column {
    type EntityName = Entity;
    fn def(&self) -> ColumnDef {
        match self {
            Self::SummaryId => ColumnType::Uuid.def(),
            Self::UserId => ColumnType::Uuid.def(),
            Self::Semester => ColumnType::String(StringLen::None).def(),
            Self::AcademicYear => ColumnType::String(StringLen::None).def(),
            Self::Gpa => ColumnType::Decimal(Some((5, 2))).def(),
            Self::Classification => ColumnType::String(StringLen::None).def().null(),
            Self::TotalCredits => ColumnType::Integer.def().null(),
            Self::TotalPassedCredits => ColumnType::Integer.def().null(),
            Self::Metadata => ColumnType::Json.def().null(),
            Self::CreatedAt => ColumnType::DateTime.def(),
            Self::UpdatedAt => ColumnType::DateTime.def(),
        }
    }
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::User => Entity::belongs_to(super::user::Entity)
                .from(Column::UserId)
                .to(super::user::Column::UserId)
                .into(),
        }
    }
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

