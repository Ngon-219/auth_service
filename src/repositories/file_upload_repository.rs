use crate::repositories::UserRepository;
use crate::static_service::DATABASE_CONNECTION;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, Set,
};
use uuid::Uuid;

pub enum FileUploadStatus {
    Pending,
    Sync,
    Failed,
    SyncDb,
    SyncBlockchain,
}

impl FileUploadStatus {
    fn as_str(&self) -> &'static str {
        match self {
            FileUploadStatus::Pending => "pending",
            FileUploadStatus::Sync => "sync",
            FileUploadStatus::Failed => "failed",
            FileUploadStatus::SyncDb => "sync_db",
            FileUploadStatus::SyncBlockchain => "sync_blockchain",
        }
    }
}

pub struct FileUploadRepository;

impl FileUploadRepository {
    pub fn new() -> Self {
        Self
    }

    pub fn get_connection(&self) -> &'static DatabaseConnection {
        DATABASE_CONNECTION
            .get()
            .expect("DATABASE_CONNECTION not set")
    }

    pub async fn create_new_file_upload(
        &self,
        file_name: &str,
        user_id: &str,
    ) -> Result<Uuid, anyhow::Error> {
        let user_id = Uuid::parse_str(user_id)?;
        let user_repo = UserRepository::new();
        let user = user_repo.find_by_id(user_id).await?;

        if user.is_none() {
            return Err(anyhow::anyhow!("User not found"));
        }

        let file_upload_history_id = Uuid::new_v4();
        let file_upload_model = crate::entities::file_upload_history::ActiveModel {
            file_upload_history_id: Set(file_upload_history_id),
            user_id: Set(user_id),
            file_name: Set(file_name.to_string()),
            status: Set(FileUploadStatus::Pending.as_str().to_string()),
            ..Default::default()
        };
        let db = self.get_connection();
        file_upload_model.insert(db).await?;
        Ok(file_upload_history_id)
    }

    pub async fn update_file_name(
        &self,
        file_upload_history_id: &str,
        new_file_name: &str,
    ) -> Result<(), anyhow::Error> {
        let db = self.get_connection();
        let file_upload_history_id = Uuid::parse_str(file_upload_history_id)?;
        
        let file_upload = crate::entities::file_upload_history::Entity::find_by_id(file_upload_history_id)
            .one(db)
            .await?;

        if let Some(mut file_upload) = file_upload {
            let mut active_model: crate::entities::file_upload_history::ActiveModel = file_upload.into();
            active_model.file_name = Set(new_file_name.to_string());
            active_model.update(db).await?;
        }

        Ok(())
    }

    pub async fn update_status_file_upload(
        &self,
        file_upload_id: &str,
        status: FileUploadStatus,
    ) -> Result<(), anyhow::Error> {
        let file_upload_id = Uuid::parse_str(file_upload_id)?;
        let db = self.get_connection();
        let find_upload_model = crate::entities::file_upload_history::Entity::find()
            .filter(
                crate::entities::file_upload_history::Column::FileUploadHistoryId
                    .eq(file_upload_id),
            )
            .one(db)
            .await?;

        let upload_model =
            find_upload_model.ok_or_else(|| anyhow::anyhow!("File upload record not found"))?;

        let mut active_model: crate::entities::file_upload_history::ActiveModel =
            upload_model.into();
        active_model.status = Set(status.as_str().to_string());

        active_model.update(db).await?;

        Ok(())
    }

    pub async fn find_by_id(
        &self,
        id: &str,
    ) -> Result<crate::entities::file_upload_history::Model, anyhow::Error> {
        let db = self.get_connection();
        let id = Uuid::parse_str(id)?;
        let find_upload_model = crate::entities::file_upload_history::Entity::find()
            .filter(crate::entities::file_upload_history::Column::FileUploadHistoryId.eq(id))
            .one(db)
            .await?;

        if let Some(find_upload_model) = find_upload_model {
            Ok(find_upload_model)
        } else {
            Err(anyhow::anyhow!("File not found"))
        }
    }

    /// Get all file uploads with pagination
    pub async fn find_all_with_pagination(
        &self,
        page: u32,
        page_size: u32,
        user_id_filter: Option<Uuid>,
        status_filter: Option<String>,
    ) -> Result<(Vec<crate::entities::file_upload_history::Model>, u64), anyhow::Error> {
        let db = self.get_connection();
        let mut query = crate::entities::file_upload_history::Entity::find();

        // Filter by user_id if provided
        if let Some(user_id) = user_id_filter {
            query = query.filter(crate::entities::file_upload_history::Column::UserId.eq(user_id));
        }

        // Filter by status if provided
        if let Some(status) = status_filter {
            query = query.filter(crate::entities::file_upload_history::Column::Status.eq(status));
        }

        // Get total count
        let total = query.clone().count(db).await?;

        // Apply pagination
        let offset = (page - 1) * page_size;
        let file_uploads = query
            .order_by_desc(crate::entities::file_upload_history::Column::CreatedAt)
            .limit(page_size as u64)
            .offset(offset as u64)
            .all(db)
            .await?;

        Ok((file_uploads, total))
    }

    /// Find file upload history by file name
    pub async fn find_by_file_name(
        &self,
        file_name: &str,
    ) -> Result<Option<crate::entities::file_upload_history::Model>, anyhow::Error> {
        let db = self.get_connection();
        let file_upload = crate::entities::file_upload_history::Entity::find()
            .filter(crate::entities::file_upload_history::Column::FileName.eq(file_name))
            .order_by_desc(crate::entities::file_upload_history::Column::CreatedAt)
            .one(db)
            .await?;
        Ok(file_upload)
    }

    /// Check and update status when create_user_db process completes (current = total)
    pub async fn check_and_update_status_on_completion(
        &self,
        file_name: &str,
        status: FileUploadStatus,
    ) -> Result<(), anyhow::Error> {
        use crate::redis_service::redis_service::helper_get_current_file_progress;
        
        // Get progress
        let progress = helper_get_current_file_progress(file_name).await?;
        
        // Check if process is complete (current >= total and total > 0)
        if progress.total > 0 && progress.current >= progress.total {
            // Find file upload history by file name
            if let Some(file_upload) = self.find_by_file_name(file_name).await? {
                // If all rows failed, update status to Failed
                let final_status = if progress.failed > 0 && progress.success == 0 {
                    FileUploadStatus::Failed
                } else {
                    status
                };
                
                let status_str = final_status.as_str();
                // Update status
                self.update_status_file_upload(
                    &file_upload.file_upload_history_id.to_string(),
                    final_status,
                )
                .await?;
                
                tracing::info!(
                    "File upload {} completed (current: {}, total: {}, success: {}, failed: {}), status updated to {}",
                    file_name,
                    progress.current,
                    progress.total,
                    progress.success,
                    progress.failed,
                    status_str
                );
            }
        }
        
        Ok(())
    }

    /// Check and update status when blockchain registration process completes (current = total)
    pub async fn check_and_update_blockchain_status_on_completion(
        &self,
        file_upload_history_id: &str,
        status: FileUploadStatus,
    ) -> Result<(), anyhow::Error> {
        use crate::redis_service::redis_service::helper_get_blockchain_registration_progress;
        
        // Get progress
        let progress = helper_get_blockchain_registration_progress(file_upload_history_id).await?;
        
        // Check if process is complete (current >= total and total > 0)
        if progress.total > 0 && progress.current >= progress.total {
            // If all rows failed, update status to Failed
            let final_status = if progress.failed > 0 && progress.success == 0 {
                FileUploadStatus::Failed
            } else {
                status
            };
            
            let status_str = final_status.as_str();
            // Update status
            self.update_status_file_upload(file_upload_history_id, final_status).await?;
            
            tracing::info!(
                "Blockchain registration for {} completed (current: {}, total: {}, success: {}, failed: {}), status updated to {}",
                file_upload_history_id,
                progress.current,
                progress.total,
                progress.success,
                progress.failed,
                status_str
            );
        }
        
        Ok(())
    }
}
