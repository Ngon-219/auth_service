use crate::entities::sea_orm_active_enums::{RoleEnum, UserStatus};
use crate::entities::{user, user_major, wallet};
use crate::static_service::DATABASE_CONNECTION;
use anyhow::Result;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DeleteResult, EntityTrait, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, Set,
};
use uuid::Uuid;

pub struct UserRepository;

impl UserRepository {
    pub fn new() -> Self {
        Self
    }

    pub fn get_connection(&self) -> &'static DatabaseConnection {
        DATABASE_CONNECTION
            .get()
            .expect("DATABASE_CONNECTION not set")
    }

    pub async fn find_by_id(&self, user_id: Uuid) -> Result<Option<user::Model>> {
        let db = self.get_connection();
        let user = user::Entity::find_by_id(user_id)
            .filter(user::Column::DeletedAt.is_null())
            .one(db)
            .await?;
        Ok(user)
    }

    pub async fn find_by_email(&self, email: &str) -> Result<Option<user::Model>> {
        let db = self.get_connection();
        let user = user::Entity::find()
            .filter(user::Column::Email.eq(email))
            .filter(user::Column::DeletedAt.is_null())
            .one(db)
            .await?;
        Ok(user)
    }

    pub async fn is_email_used_by_sync_user(&self, email: &str) -> Result<bool> {
        let db = self.get_connection();
        let count = user::Entity::find()
            .filter(user::Column::Email.eq(email))
            .filter(user::Column::DeletedAt.is_null())
            .filter(user::Column::Status.eq(UserStatus::Sync))
            .count(db)
            .await?;
        Ok(count > 0)
    }

    pub async fn find_all_with_pagination(
        &self,
        page: u32,
        page_size: u32,
        role_filter: Option<RoleEnum>,
        search: Option<String>,
        manager_only_students: bool,
    ) -> Result<(Vec<user::Model>, u64)> {
        let db = self.get_connection();
        let mut query = user::Entity::find().filter(user::Column::DeletedAt.is_null());

        // Manager only sees students
        if manager_only_students {
            query = query.filter(user::Column::Role.eq(RoleEnum::Student));
        }

        // Filter by role if provided
        if let Some(role) = role_filter {
            query = query.filter(user::Column::Role.eq(role));
        }

        // Search by name or email
        if let Some(search_term) = search {
            let search_pattern = format!("%{}%", search_term);
            query = query.filter(
                user::Column::FirstName
                    .contains(&search_pattern)
                    .or(user::Column::LastName.contains(&search_pattern))
                    .or(user::Column::Email.contains(&search_pattern))
                    .or(user::Column::StudentCode.contains(&search_pattern)),
            );
        }

        // Get total count
        let total = query.clone().count(db).await?;

        // Apply pagination
        let offset = (page - 1) * page_size;
        let users = query
            .order_by_desc(user::Column::CreateAt)
            .limit(page_size as u64)
            .offset(offset as u64)
            .all(db)
            .await?;

        Ok((users, total))
    }

    pub async fn get_user_with_wallet_and_majors(
        &self,
        user_id: Uuid,
    ) -> Result<Option<(user::Model, Option<wallet::Model>, Vec<Uuid>)>> {
        // find_by_id already filters deleted_at IS NULL
        let user = self.find_by_id(user_id).await?;

        if let Some(user_model) = user {
            let db = self.get_connection();
            // Get wallet info
            let wallet_info = wallet::Entity::find()
                .filter(wallet::Column::UserId.eq(user_id))
                .one(db)
                .await?;

            // Get major IDs
            let major_relationships = user_major::Entity::find()
                .filter(user_major::Column::UserId.eq(user_id))
                .all(db)
                .await?;

            let major_ids = major_relationships
                .into_iter()
                .map(|m| m.major_id)
                .collect();

            Ok(Some((user_model, wallet_info, major_ids)))
        } else {
            Ok(None)
        }
    }

    pub async fn create(
        &self,
        user_id: Uuid,
        first_name: String,
        last_name: String,
        address: String,
        email: String,
        password: String,
        cccd: String,
        phone_number: String,
        role: RoleEnum,
        is_priority: bool,
        student_code: Option<String>,
    ) -> Result<user::Model> {
        let db = self.get_connection();
        let now = chrono::Utc::now().naive_utc();
        let mut user_model = user::ActiveModel {
            user_id: Set(user_id),
            first_name: Set(first_name),
            last_name: Set(last_name),
            address: Set(address),
            email: Set(email),
            password: Set(password),
            is_priority: Set(is_priority),
            cccd: Set(cccd),
            phone_number: Set(phone_number),
            is_first_login: Set(true),
            create_at: Set(now),
            update_at: Set(now),
            role: Set(role),
            ..Default::default()
        };

        // Set student_code if provided (entity must have this field)
        if let Some(code) = student_code {
            user_model.student_code = Set(Some(code));
        }

        let result = user_model.insert(db).await?;
        Ok(result)
    }

    pub async fn update(&self, user_id: Uuid, updates: UserUpdate) -> Result<user::Model> {
        let user = self
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("User not found"))?;
        let db = self.get_connection();

        let mut active_user: user::ActiveModel = user.into();
        let now = chrono::Utc::now().naive_utc();

        if let Some(first_name) = updates.first_name {
            active_user.first_name = Set(first_name);
        }
        if let Some(last_name) = updates.last_name {
            active_user.last_name = Set(last_name);
        }
        if let Some(address) = updates.address {
            active_user.address = Set(address);
        }
        if let Some(email) = updates.email {
            active_user.email = Set(email);
        }
        if let Some(password) = updates.password {
            active_user.password = Set(password);
        }
        if let Some(cccd) = updates.cccd {
            active_user.cccd = Set(cccd);
        }
        if let Some(phone_number) = updates.phone_number {
            active_user.phone_number = Set(phone_number);
        }
        if let Some(role) = updates.role {
            active_user.role = Set(role);
        }
        if let Some(is_priority) = updates.is_priority {
            active_user.is_priority = Set(is_priority);
        }
        if let Some(is_first_login) = updates.is_first_login {
            active_user.is_first_login = Set(is_first_login);
        }

        active_user.update_at = Set(now);

        let result = active_user.update(db).await?;
        Ok(result)
    }

    pub async fn delete(&self, user_id: Uuid) -> Result<DeleteResult> {
        let db = self.get_connection();
        // Delete user major relationships first (foreign key constraint)
        user_major::Entity::delete_many()
            .filter(user_major::Column::UserId.eq(user_id))
            .exec(db)
            .await?;

        // Delete user
        let result = user::Entity::delete_by_id(user_id).exec(db).await?;

        Ok(result)
    }

    /// Soft delete user by setting deleted_at timestamp
    pub async fn soft_delete(&self, user_id: Uuid) -> Result<user::Model> {
        let db = self.get_connection();
        let user = user::Entity::find_by_id(user_id)
            .one(db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("User not found"))?;

        let mut active_user: user::ActiveModel = user.into();
        let now = chrono::Utc::now().naive_utc();

        // Set deleted_at to mark as soft deleted
        active_user.deleted_at = Set(Some(now));
        active_user.update_at = Set(now);

        let result = active_user.update(db).await?;
        Ok(result)
    }

    pub async fn delete_by_student_code(&self, student_code: &str) -> Result<DeleteResult> {
        let db = self.get_connection();
        let user = user::Entity::find()
            .filter(user::Column::StudentCode.eq(student_code))
            .one(db)
            .await?;

        if let Some(user_model) = user {
            let user_id = user_model.user_id;
            user_major::Entity::delete_many()
                .filter(user_major::Column::UserId.eq(user_id))
                .exec(db)
                .await?;

            let result = user::Entity::delete_by_id(user_id).exec(db).await?;

            Ok(result)
        } else {
            Err(anyhow::anyhow!("User not found"))
        }
    }

    pub async fn delete_by_email(&self, email: &str) -> Result<DeleteResult> {
        let db = self.get_connection();
        let user = user::Entity::find()
            .filter(user::Column::Email.eq(email))
            .one(db)
            .await?;

        if let Some(user_model) = user {
            let user_id = user_model.user_id;
            user_major::Entity::delete_many()
                .filter(user_major::Column::UserId.eq(user_id))
                .exec(db)
                .await?;

            let result = user::Entity::delete_by_id(user_id).exec(db).await?;

            Ok(result)
        } else {
            Err(anyhow::anyhow!("User not found"))
        }
    }

    pub async fn get_latest_student_code() -> Result<String> {
        let db = UserRepository.get_connection();
        let user = user::Entity::find()
            // .filter(user::Column::DeletedAt.is_null())
            .filter(user::Column::Role.eq(RoleEnum::Student))
            .order_by_desc(user::Column::CreateAt)
            .one(db)
            .await?;

        if let Some(user) = user {
            return Ok(user.student_code.unwrap_or("000000".to_string()));
        }

        Err(anyhow::anyhow!("User not found"))
    }

    /// Get students by their email addresses
    /// This is used to identify students created from a specific CSV file upload
    pub async fn find_students_by_emails(
        &self,
        emails: Vec<String>,
    ) -> Result<Vec<(user::Model, wallet::Model)>> {
        let db = self.get_connection();

        if emails.is_empty() {
            return Ok(Vec::new());
        }

        // Query students by emails
        let students = user::Entity::find()
            .filter(user::Column::DeletedAt.is_null())
            .filter(user::Column::Role.eq(RoleEnum::Student))
            .filter(user::Column::Email.is_in(emails))
            .all(db)
            .await?;

        let mut result = Vec::new();
        for student in students {
            let wallet_info = wallet::Entity::find()
                .filter(wallet::Column::UserId.eq(student.user_id))
                .one(db)
                .await?;

            if let Some(wallet) = wallet_info {
                result.push((student, wallet));
            }
        }

        Ok(result)
    }

    pub async fn update_status_by_email(
        &self,
        email: &str,
        status: UserStatus,
    ) -> Result<user::Model> {
        let db = self.get_connection();
        let user = user::Entity::find()
            .filter(user::Column::Email.eq(email))
            .filter(user::Column::DeletedAt.is_null())
            .order_by_desc(user::Column::CreateAt)
            .one(db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("User not found for email {}", email))?;

        let mut active_user: user::ActiveModel = user.into();
        active_user.status = Set(status);
        active_user.update_at = Set(chrono::Utc::now().naive_utc());

        let result = active_user.update(db).await?;
        Ok(result)
    }
}

#[derive(Default)]
pub struct UserUpdate {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub address: Option<String>,
    pub email: Option<String>,
    pub password: Option<String>,
    pub cccd: Option<String>,
    pub phone_number: Option<String>,
    pub role: Option<RoleEnum>,
    pub is_priority: Option<bool>,
    pub is_first_login: Option<bool>,
}
