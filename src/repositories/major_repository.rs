use sea_orm::{DatabaseConnection, EntityTrait, ColumnTrait, QueryFilter, ActiveModelTrait, Set, DeleteResult};
use uuid::Uuid;
use crate::entities::major;
use crate::static_service::DATABASE_CONNECTION;
use anyhow::Result;
use chrono::{NaiveDateTime, Utc};

pub struct MajorRepository;

impl MajorRepository {
    pub fn new() -> Self {
        Self
    }

    fn get_connection(&self) -> &'static DatabaseConnection {
        DATABASE_CONNECTION
            .get()
            .expect("DATABASE_CONNECTION not set")
    }

    pub async fn find_all(&self) -> Result<Vec<major::Model>> {
        let db = self.get_connection();
        let majors = major::Entity::find()
            .all(db)
            .await?;
        Ok(majors)
    }

    pub async fn find_by_id(
        &self,
        major_id: Uuid,
    ) -> Result<Option<major::Model>> {
        let db = self.get_connection();
        let major = major::Entity::find()
            .filter(major::Column::MajorId.eq(major_id))
            .one(db)
            .await?;
        Ok(major)
    }

    pub async fn create(
        &self,
        major_id: Uuid,
        name: String,
        founding_date: NaiveDateTime,
        department_id: Option<Uuid>,
    ) -> Result<major::Model> {
        let db = self.get_connection();
        let now = Utc::now().naive_utc();
        let major_model = major::ActiveModel {
            major_id: Set(major_id),
            name: Set(name),
            founding_date: Set(founding_date),
            department_id: Set(department_id),
            create_at: Set(now),
            update_at: Set(now),
        };

        let result = major_model.insert(db).await?;
        Ok(result)
    }

    pub async fn update(
        &self,
        major_id: Uuid,
        updates: MajorUpdate,
    ) -> Result<major::Model> {
        let major = self.find_by_id(major_id).await?
            .ok_or_else(|| anyhow::anyhow!("Major not found"))?;
        let db = self.get_connection();

        let mut active_model: major::ActiveModel = major.into();

        if let Some(name) = updates.name {
            active_model.name = Set(name);
        }
        if let Some(founding_date) = updates.founding_date {
            active_model.founding_date = Set(founding_date);
        }
        if let Some(department_id) = updates.department_id {
            active_model.department_id = Set(Some(department_id));
        }

        active_model.update_at = Set(Utc::now().naive_utc());

        let result = active_model.update(db).await?;
        Ok(result)
    }

    pub async fn delete(
        &self,
        major_id: Uuid,
    ) -> Result<DeleteResult> {
        let major = self.find_by_id(major_id).await?
            .ok_or_else(|| anyhow::anyhow!("Major not found"))?;
        let db = self.get_connection();

        let active_model: major::ActiveModel = major.into();
        let result = active_model.delete(db).await?;
        Ok(result)
    }
}

pub struct MajorUpdate {
    pub name: Option<String>,
    pub founding_date: Option<NaiveDateTime>,
    pub department_id: Option<Uuid>,
}
