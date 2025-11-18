use crate::entities::{request, sea_orm_active_enums::RequestStatus};
use crate::static_service::DATABASE_CONNECTION;
use anyhow::Result;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, Set,
};
use uuid::Uuid;

pub struct RequestRepository;

impl RequestRepository {
    pub fn new() -> Self {
        Self
    }

    pub fn get_connection(&self) -> &'static DatabaseConnection {
        DATABASE_CONNECTION
            .get()
            .expect("DATABASE_CONNECTION not set")
    }

    /// Create a new request
    pub async fn create(&self, user_id: Uuid, content: String) -> Result<request::Model> {
        let db = self.get_connection();

        let request = request::ActiveModel {
            user_id: Set(user_id),
            content: Set(content),
            status: Set(RequestStatus::Pending),
            scheduled_at: Set(None),
            ..Default::default()
        };

        let result = request.insert(db).await?;
        Ok(result)
    }

    /// Get request by ID
    pub async fn find_by_id(&self, request_id: Uuid) -> Result<Option<request::Model>> {
        let db = self.get_connection();
        let request = request::Entity::find_by_id(request_id).one(db).await?;
        Ok(request)
    }

    /// Get all requests for a user
    pub async fn find_by_user_id(&self, user_id: Uuid) -> Result<Vec<request::Model>> {
        let db = self.get_connection();
        let requests = request::Entity::find()
            .filter(request::Column::UserId.eq(user_id))
            .order_by_desc(request::Column::CreatedAt)
            .all(db)
            .await?;
        Ok(requests)
    }

    /// Get all requests (for managers/admins) with pagination
    pub async fn find_all_with_pagination(
        &self,
        page: u32,
        page_size: u32,
        status_filter: Option<RequestStatus>,
    ) -> Result<(Vec<request::Model>, u64)> {
        let db = self.get_connection();
        let mut query = request::Entity::find();

        // Filter by status if provided
        if let Some(status) = status_filter {
            query = query.filter(request::Column::Status.eq(status));
        }

        // Get total count
        let total = query.clone().count(db).await?;

        // Apply pagination
        let offset = (page - 1) * page_size;
        let requests = query
            .order_by_desc(request::Column::CreatedAt)
            .limit(page_size as u64)
            .offset(offset as u64)
            .all(db)
            .await?;

        Ok((requests, total))
    }

    /// Get all requests (for managers/admins) - deprecated, use find_all_with_pagination
    pub async fn find_all(&self) -> Result<Vec<request::Model>> {
        let db = self.get_connection();
        let requests = request::Entity::find()
            .order_by_desc(request::Column::CreatedAt)
            .all(db)
            .await?;
        Ok(requests)
    }

    /// Update request status and scheduled_at
    pub async fn update_status_and_schedule(
        &self,
        request_id: Uuid,
        status: RequestStatus,
        scheduled_at: Option<chrono::NaiveDateTime>,
    ) -> Result<request::Model> {
        let db = self.get_connection();
        let now = Utc::now().naive_utc();

        let mut request: request::ActiveModel = request::Entity::find_by_id(request_id)
            .one(db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Request not found"))?
            .into();

        request.status = Set(status);
        request.scheduled_at = Set(scheduled_at);
        request.updated_at = Set(now);

        let result = request.update(db).await?;
        Ok(result)
    }

    /// Update request status to rejected
    pub async fn reject_request(&self, request_id: Uuid) -> Result<request::Model> {
        self.update_status_and_schedule(request_id, RequestStatus::Rejected, None)
            .await
    }
}
