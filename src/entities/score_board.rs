//! `SeaORM` Entity for score_board table

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Copy, Clone, Default, Debug, DeriveEntity)]
pub struct Entity;

impl EntityName for Entity {
    fn table_name(&self) -> &str {
        "score_board"
    }
}

#[derive(Clone, Debug, PartialEq, DeriveModel, DeriveActiveModel, Eq, Serialize, Deserialize)]
pub struct Model {
    #[serde(skip_deserializing)]
    pub score_board_id: Uuid,
    pub user_id: Uuid,
    pub course_id: String,
    pub course_name: String,
    pub course_code: Option<String>,
    pub credits: i32,
    pub score1: Option<Decimal>,
    pub score2: Option<Decimal>,
    pub score3: Option<Decimal>,
    pub score4: Option<Decimal>,
    pub score5: Option<Decimal>,
    pub score6: Option<Decimal>,
    pub letter_grade: Option<String>,
    pub status: Option<String>,
    pub semester: String,
    pub academic_year: Option<String>,
    pub metadata: Option<Value>,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveColumn)]
pub enum Column {
    ScoreBoardId,
    UserId,
    CourseId,
    CourseName,
    CourseCode,
    Credits,
    Score1,
    Score2,
    Score3,
    Score4,
    Score5,
    Score6,
    LetterGrade,
    Status,
    Semester,
    AcademicYear,
    Metadata,
    CreatedAt,
    UpdatedAt,
}

#[derive(Copy, Clone, Debug, EnumIter, DerivePrimaryKey)]
pub enum PrimaryKey {
    ScoreBoardId,
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
            Self::ScoreBoardId => ColumnType::Uuid.def(),
            Self::UserId => ColumnType::Uuid.def(),
            Self::CourseId => ColumnType::String(StringLen::None).def(),
            Self::CourseName => ColumnType::String(StringLen::None).def(),
            Self::CourseCode => ColumnType::String(StringLen::None).def().null(),
            Self::Credits => ColumnType::Integer.def(),
            Self::Score1 => ColumnType::Decimal(Some((5, 2))).def().null(),
            Self::Score2 => ColumnType::Decimal(Some((5, 2))).def().null(),
            Self::Score3 => ColumnType::Decimal(Some((5, 2))).def().null(),
            Self::Score4 => ColumnType::Decimal(Some((5, 2))).def().null(),
            Self::Score5 => ColumnType::Decimal(Some((5, 2))).def().null(),
            Self::Score6 => ColumnType::Decimal(Some((5, 2))).def().null(),
            Self::LetterGrade => ColumnType::String(StringLen::None).def().null(),
            Self::Status => ColumnType::String(StringLen::None).def().null(),
            Self::Semester => ColumnType::String(StringLen::None).def(),
            Self::AcademicYear => ColumnType::String(StringLen::None).def().null(),
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

