use axum::{
    extract::Query,
    http::StatusCode,
    routing::get,
    Json, Router,
};
use chrono::NaiveDateTime;
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QuerySelect};

use crate::entities::{documents, user};
use crate::entities::sea_orm_active_enums::RoleEnum;
use crate::extractor::AuthClaims;
use crate::static_service::DATABASE_CONNECTION;
use do_an_lib::structs::token_claims::UserRole;

use super::dto::{DateRangeQuery, DocumentStatsResponse, TimeSeriesPoint, UserStatsResponse};

pub fn create_route() -> Router {
    Router::new()
        .route("/api/v1/stats/users", get(get_user_stats))
        .route("/api/v1/stats/documents", get(get_document_stats))
}

fn to_date(dt: NaiveDateTime) -> String {
    dt.date().format("%Y-%m-%d").to_string()
}

#[utoipa::path(
    get,
    path = "/api/v1/stats/users",
    params(DateRangeQuery),
    responses(
        (status = 200, description = "User statistics", body = UserStatsResponse),
        (status = 400, description = "Invalid date range"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Statistics"
)]
pub async fn get_user_stats(
    AuthClaims(claims): AuthClaims,
    Query(query): Query<DateRangeQuery>,
) -> Result<(StatusCode, Json<UserStatsResponse>), (StatusCode, String)> {
    // Only admin/manager can view system stats
    if claims.role != UserRole::ADMIN && claims.role != UserRole::MANAGER {
        return Err((StatusCode::FORBIDDEN, "Forbidden".to_string()));
    }

    let (start, end) = query
        .to_range()
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    let db = DATABASE_CONNECTION
        .get()
        .expect("DATABASE_CONNECTION not set");

    // Total counts by role
    let total_users = user::Entity::find()
        .filter(user::Column::DeletedAt.is_null())
        .count(db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to count users: {}", e),
            )
        })? as i64;

    let total_students = user::Entity::find()
        .filter(user::Column::DeletedAt.is_null())
        .filter(user::Column::Role.eq(RoleEnum::Student))
        .count(db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to count students: {}", e),
            )
        })? as i64;

    let total_managers = user::Entity::find()
        .filter(user::Column::DeletedAt.is_null())
        .filter(user::Column::Role.eq(RoleEnum::Manager))
        .count(db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to count managers: {}", e),
            )
        })? as i64;

    let total_teachers = user::Entity::find()
        .filter(user::Column::DeletedAt.is_null())
        .filter(user::Column::Role.eq(RoleEnum::Teacher))
        .count(db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to count teachers: {}", e),
            )
        })? as i64;

    let total_admins = user::Entity::find()
        .filter(user::Column::DeletedAt.is_null())
        .filter(user::Column::Role.eq(RoleEnum::Admin))
        .count(db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to count admins: {}", e),
            )
        })? as i64;

    // Users created per day in range
    let rows = user::Entity::find()
        .filter(user::Column::DeletedAt.is_null())
        .filter(user::Column::CreateAt.gte(start))
        .filter(user::Column::CreateAt.lte(end))
        .select_only()
        .column_as(user::Column::CreateAt, "created_at")
        .into_tuple::<NaiveDateTime>()
        .all(db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to query users per day: {}", e),
            )
        })?;

    let mut map = std::collections::BTreeMap::<String, i64>::new();
    for created_at in rows {
        let key = to_date(created_at);
        *map.entry(key).or_insert(0) += 1;
    }

    let users_per_day = map
        .into_iter()
        .map(|(date, count)| TimeSeriesPoint { date, count })
        .collect();

    Ok((
        StatusCode::OK,
        Json(UserStatsResponse {
            total_users,
            total_students,
            total_managers,
            total_teachers,
            total_admins,
            users_per_day,
        }),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/stats/documents",
    params(DateRangeQuery),
    responses(
        (status = 200, description = "Document statistics", body = DocumentStatsResponse),
        (status = 400, description = "Invalid date range"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Statistics"
)]
pub async fn get_document_stats(
    AuthClaims(claims): AuthClaims,
    Query(query): Query<DateRangeQuery>,
) -> Result<(StatusCode, Json<DocumentStatsResponse>), (StatusCode, String)> {
    if claims.role != UserRole::ADMIN && claims.role != UserRole::MANAGER {
        return Err((StatusCode::FORBIDDEN, "Forbidden".to_string()));
    }

    let (start, end) = query
        .to_range()
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    let db = DATABASE_CONNECTION
        .get()
        .expect("DATABASE_CONNECTION not set");

    // Total documents
    let total_documents = documents::Entity::find()
        .count(db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to count documents: {}", e),
            )
        })? as i64;

    // Signed documents on-chain (status = 'minted')
    let signed_documents = documents::Entity::find()
        .filter(documents::Column::Status.eq("minted"))
        .count(db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to count signed documents: {}", e),
            )
        })? as i64;

    // Failed (status = 'failed')
    let failed_documents = documents::Entity::find()
        .filter(documents::Column::Status.eq("failed"))
        .count(db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to count failed documents: {}", e),
            )
        })? as i64;

    // Documents per day in range, based on created_at
    let rows = documents::Entity::find()
        .filter(documents::Column::CreatedAt.gte(start))
        .filter(documents::Column::CreatedAt.lte(end))
        .select_only()
        .column_as(documents::Column::CreatedAt, "created_at")
        .into_tuple::<NaiveDateTime>()
        .all(db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to query documents per day: {}", e),
            )
        })?;

    let mut map = std::collections::BTreeMap::<String, i64>::new();
    for created_at in rows {
        let key = to_date(created_at);
        *map.entry(key).or_insert(0) += 1;
    }

    let documents_per_day = map
        .into_iter()
        .map(|(date, count)| TimeSeriesPoint { date, count })
        .collect();

    Ok((
        StatusCode::OK,
        Json(DocumentStatsResponse {
            total_documents,
            signed_documents,
            failed_documents,
            documents_per_day,
        }),
    ))
}


