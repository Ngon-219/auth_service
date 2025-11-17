//! `SeaORM` Entity for certificate table

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Copy, Clone, Default, Debug, DeriveEntity)]
pub struct Entity;

impl EntityName for Entity {
    fn table_name(&self) -> &str {
        "certificate"
    }
}

#[derive(Clone, Debug, PartialEq, DeriveModel, DeriveActiveModel, Eq, Serialize, Deserialize)]
pub struct Model {
    #[serde(skip_deserializing)]
    pub certificate_id: Uuid,
    pub user_id: Uuid,
    pub certificate_type: String,
    pub issued_date: Date,
    pub expiry_date: Option<Date>,
    pub description: Option<String>,
    pub metadata: Option<Value>,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveColumn)]
pub enum Column {
    CertificateId,
    UserId,
    CertificateType,
    IssuedDate,
    ExpiryDate,
    Description,
    Metadata,
    CreatedAt,
    UpdatedAt,
}

#[derive(Copy, Clone, Debug, EnumIter, DerivePrimaryKey)]
pub enum PrimaryKey {
    CertificateId,
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
            Self::CertificateId => ColumnType::Uuid.def(),
            Self::UserId => ColumnType::Uuid.def(),
            Self::CertificateType => ColumnType::String(StringLen::None).def(),
            Self::IssuedDate => ColumnType::Date.def(),
            Self::ExpiryDate => ColumnType::Date.def().null(),
            Self::Description => ColumnType::Text.def().null(),
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

