use sea_orm::{DatabaseConnection, EntityTrait, ColumnTrait, QueryFilter, ActiveModelTrait, Set, DeleteResult};
use uuid::Uuid;
use crate::entities::department;
use crate::static_service::DATABASE_CONNECTION;
use anyhow::Result;
use chrono::{NaiveDateTime, Utc};

pub struct DepartmentRepository;

impl DepartmentRepository {
    pub fn new() -> Self {
        Self
    }

    fn get_connection(&self) -> &'static DatabaseConnection {
        DATABASE_CONNECTION
            .get()
            .expect("DATABASE_CONNECTION not set")
    }

    pub async fn find_all(&self) -> Result<Vec<department::Model>> {
        let db = self.get_connection();
        let departments = department::Entity::find()
            .all(db)
            .await?;
        Ok(departments)
    }

    pub async fn find_by_id(
        &self,
        department_id: Uuid,
    ) -> Result<Option<department::Model>> {
        let db = self.get_connection();
        let department = department::Entity::find()
            .filter(department::Column::DepartmentId.eq(department_id))
            .one(db)
            .await?;
        Ok(department)
    }

    pub async fn create(
        &self,
        department_id: Uuid,
        name: String,
        founding_date: NaiveDateTime,
        dean: String,
    ) -> Result<department::Model> {
        let db = self.get_connection();
        let now = Utc::now().naive_utc();
        let department_model = department::ActiveModel {
            department_id: Set(department_id),
            name: Set(name),
            founding_date: Set(founding_date),
            dean: Set(dean),
            create_at: Set(now),
            update_at: Set(now),
        };

        let result = department_model.insert(db).await?;
        Ok(result)
    }

    pub async fn update(
        &self,
        department_id: Uuid,
        updates: DepartmentUpdate,
    ) -> Result<department::Model> {
        let department = self.find_by_id(department_id).await?
            .ok_or_else(|| anyhow::anyhow!("Department not found"))?;
        let db = self.get_connection();

        let mut active_model: department::ActiveModel = department.into();

        if let Some(name) = updates.name {
            active_model.name = Set(name);
        }
        if let Some(founding_date) = updates.founding_date {
            active_model.founding_date = Set(founding_date);
        }
        if let Some(dean) = updates.dean {
            active_model.dean = Set(dean);
        }

        active_model.update_at = Set(Utc::now().naive_utc());

        let result = active_model.update(db).await?;
        Ok(result)
    }

    pub async fn delete(
        &self,
        department_id: Uuid,
    ) -> Result<DeleteResult> {
        let department = self.find_by_id(department_id).await?
            .ok_or_else(|| anyhow::anyhow!("Department not found"))?;
        let db = self.get_connection();

        let active_model: department::ActiveModel = department.into();
        let result = active_model.delete(db).await?;
        Ok(result)
    }
}

pub struct DepartmentUpdate {
    pub name: Option<String>,
    pub founding_date: Option<NaiveDateTime>,
    pub dean: Option<String>,
}
