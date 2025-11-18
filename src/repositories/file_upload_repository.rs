use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use uuid::Uuid;
use crate::repositories::UserRepository;
use crate::static_service::DATABASE_CONNECTION;


pub enum FileUploadStatus {
    Pending,
    Sync,
    Failed,
}

impl FileUploadStatus {
    fn as_str(&self) -> &'static str {
        match self {
            FileUploadStatus::Pending => "pending",
            FileUploadStatus::Sync => "sync",
            FileUploadStatus::Failed => "failed",
        }
    }
}

pub struct FileUploadRepository;

impl FileUploadRepository {
    pub fn new() -> Self{Self}

    pub fn get_connection(&self) -> &'static DatabaseConnection {
        DATABASE_CONNECTION
            .get()
            .expect("DATABASE_CONNECTION not set")
    }

    pub async fn create_new_file_upload(&self, file_name: &str, user_id: &str) -> Result<(), anyhow::Error>{
        let user_id = Uuid::parse_str(user_id)?;
        let user_repo = UserRepository::new();
        let user = user_repo.find_by_id(user_id).await?;

        if user.is_none() {
            return Err(anyhow::anyhow!("User not found"));
        }

        let file_upload_model = crate::entities::file_upload_history::ActiveModel {
            user_id: Set(user_id),
            file_name: Set(file_name.to_string()),
            status: Set(FileUploadStatus::Pending.as_str().to_string()),
            ..Default::default()
        };
        let db = self.get_connection();
        file_upload_model.insert(db).await?;
        Ok(())
    }

    pub async fn update_status_file_upload(&self, file_upload_id: &str, status: FileUploadStatus) -> Result<(), anyhow::Error> {
        let file_upload_id = Uuid::parse_str(file_upload_id)?;
        let db = self.get_connection();
        let find_upload_model = crate::entities::file_upload_history::Entity::find()
            .filter(crate::entities::file_upload_history::Column::FileUploadHistoryId.eq(file_upload_id))
            .one(db)
            .await?;

        let upload_model = find_upload_model.ok_or_else(|| anyhow::anyhow!("File upload record not found"))?;

        let mut active_model: crate::entities::file_upload_history::ActiveModel = upload_model.into();
        active_model.status = Set(status.as_str().to_string());

        active_model.update(db).await?;

        Ok(())
    }
}