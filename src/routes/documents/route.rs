use super::dto::{
    CertificateItem, DocumentData, DocumentDataRequest, DocumentDataResponse,
    DocumentTypeResponse, MockCertificateRequest, MockCertificateResponse, MockTranscriptRequest,
    MockTranscriptResponse, ScoreBoardItem, SemesterSummaryItem, UpdateDocumentTypeRequest,
    UpdateDocumentTypeResponse, UserInfo,
};
use crate::entities::{document_type, sea_orm_active_enums::RoleEnum};
use crate::extractor::AuthClaims;
use crate::repositories::{ScoreRepository, UserRepository};
use crate::static_service::DATABASE_CONNECTION;
use axum::{extract::Path, http::StatusCode, routing::{get, post, put}, Json, Router};
use chrono::NaiveDate;
use do_an_lib::structs::token_claims::UserRole;
use sea_orm::{EntityTrait, ActiveModelTrait, Set};
use uuid::Uuid;

pub fn create_route() -> Router {
    Router::new()
        .route("/api/v1/documents/data", post(get_document_data))
        .route("/api/v1/documents/types", get(get_document_types))
        .route("/api/v1/documents/types/{document_type_id}", get(get_document_type_by_id).put(update_document_type))
        .route("/api/v1/documents/mock/certificate", post(mock_certificate))
        .route("/api/v1/documents/mock/transcript", post(mock_transcript))
}

#[utoipa::path(
    get,
    path = "/api/v1/documents/types",
    responses(
        (status = 200, description = "List of document types", body = [DocumentTypeResponse]),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Documents"
)]
pub async fn get_document_types(
    AuthClaims(_auth_claims): AuthClaims,
) -> Result<(StatusCode, Json<Vec<DocumentTypeResponse>>), (StatusCode, String)> {
    let db = DATABASE_CONNECTION
        .get()
        .expect("DATABASE_CONNECTION not set");

    let types = document_type::Entity::find()
        .all(db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to load document types: {}", e),
            )
        })?;

    let response = types
        .into_iter()
        .map(|doc_type| DocumentTypeResponse {
            document_type_id: doc_type.document_type_id,
            document_type_name: doc_type.document_type_name,
            description: doc_type.description,
            template_pdf: doc_type.template_pdf,
            created_at: doc_type.created_at,
            updated_at: doc_type.updated_at,
            created_by: doc_type.created_by,
        })
        .collect();

    Ok((StatusCode::OK, Json(response)))
}

#[utoipa::path(
    get,
    path = "/api/v1/documents/types/{document_type_id}",
    params(
        ("document_type_id" = Uuid, Path, description = "Document Type ID")
    ),
    responses(
        (status = 200, description = "Document type retrieved successfully", body = DocumentTypeResponse),
        (status = 404, description = "Document type not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Documents"
)]
pub async fn get_document_type_by_id(
    AuthClaims(_auth_claims): AuthClaims,
    Path(document_type_id): Path<Uuid>,
) -> Result<(StatusCode, Json<DocumentTypeResponse>), (StatusCode, String)> {
    let db = DATABASE_CONNECTION
        .get()
        .expect("DATABASE_CONNECTION not set");

    let doc_type = document_type::Entity::find_by_id(document_type_id)
        .one(db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to load document type: {}", e),
            )
        })?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Document type not found".to_string()))?;

    let response = DocumentTypeResponse {
        document_type_id: doc_type.document_type_id,
        document_type_name: doc_type.document_type_name,
        description: doc_type.description,
        template_pdf: doc_type.template_pdf,
        created_at: doc_type.created_at,
        updated_at: doc_type.updated_at,
        created_by: doc_type.created_by,
    };

    Ok((StatusCode::OK, Json(response)))
}

#[utoipa::path(
    put,
    path = "/api/v1/documents/types/{document_type_id}",
    params(
        ("document_type_id" = Uuid, Path, description = "Document Type ID")
    ),
    request_body = UpdateDocumentTypeRequest,
    responses(
        (status = 200, description = "Document type updated successfully", body = UpdateDocumentTypeResponse),
        (status = 404, description = "Document type not found"),
        (status = 403, description = "Forbidden - Admin only"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Documents"
)]
pub async fn update_document_type(
    AuthClaims(claims): AuthClaims,
    Path(document_type_id): Path<Uuid>,
    Json(payload): Json<UpdateDocumentTypeRequest>,
) -> Result<(StatusCode, Json<UpdateDocumentTypeResponse>), (StatusCode, String)> {
    // Permission check: Admin only
    if claims.role != UserRole::ADMIN {
        return Err((
            StatusCode::FORBIDDEN,
            "Only admin can update document types".to_string(),
        ));
    }

    let db = DATABASE_CONNECTION
        .get()
        .expect("DATABASE_CONNECTION not set");

    // Find document type
    let document_type = document_type::Entity::find_by_id(document_type_id)
        .one(db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to find document type: {}", e),
            )
        })?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Document type not found".to_string()))?;

    // Update document type
    let mut active_model: document_type::ActiveModel = document_type.into();

    if let Some(document_type_name) = payload.document_type_name {
        active_model.document_type_name = Set(document_type_name);
    }

    if let Some(description) = payload.description {
        active_model.description = Set(Some(description));
    }

    if let Some(template_pdf) = payload.template_pdf {
        active_model.template_pdf = Set(Some(template_pdf));
    }

    // Update updated_at timestamp
    active_model.updated_at = Set(chrono::Utc::now().naive_utc());

    active_model
        .update(db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to update document type: {}", e),
            )
        })?;

    Ok((
        StatusCode::OK,
        Json(UpdateDocumentTypeResponse {
            document_type_id,
            message: "Document type updated successfully".to_string(),
        }),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/documents/data",
    request_body = DocumentDataRequest,
    responses(
        (status = 200, description = "Document data retrieved successfully", body = DocumentDataResponse),
        (status = 404, description = "No data found for this document type"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Documents"
)]
pub async fn get_document_data(
    AuthClaims(auth_claims): AuthClaims,
    Json(payload): Json<DocumentDataRequest>,
) -> Result<(StatusCode, Json<DocumentDataResponse>), (StatusCode, String)> {
    let user_id = uuid::Uuid::parse_str(&auth_claims.user_id).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Invalid user_id: {}", e),
        )
    })?;

    // Get user information
    let user_repo = UserRepository::new();
    let user = user_repo
        .find_by_id(user_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get user: {}", e),
            )
        })?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "User not found".to_string()))?;

    let user_info = UserInfo {
        user_id: user.user_id.to_string(),
        first_name: user.first_name,
        last_name: user.last_name,
        email: user.email,
        student_code: user.student_code,
        role: match user.role {
            RoleEnum::Admin => "admin".to_string(),
            RoleEnum::Manager => "manager".to_string(),
            RoleEnum::Teacher => "teacher".to_string(),
            RoleEnum::Student => "student".to_string(),
        },
    };

    let score_repo = ScoreRepository::new();
    let document_type_name = payload.document_type_name.trim();
    let document_type_name_lower = document_type_name.to_lowercase();

    // Determine what data to fetch based on document type
    // Standard types: Certificate, Transcript, Diploma
    let is_transcript = document_type_name.eq_ignore_ascii_case("Transcript")
        || document_type_name_lower.contains("bảng điểm")
        || document_type_name_lower.contains("transcript")
        || document_type_name_lower.contains("bang diem");

    let is_diploma = document_type_name.eq_ignore_ascii_case("Diploma")
        || document_type_name_lower.contains("bằng tốt nghiệp")
        || document_type_name_lower.contains("diploma")
        || document_type_name_lower.contains("bang tot nghiep");

    let is_certificate = document_type_name.eq_ignore_ascii_case("Certificate")
        || document_type_name_lower.contains("chứng chỉ")
        || document_type_name_lower.contains("certificate")
        || document_type_name_lower.contains("chung chi");

    let mut scoreboard_data: Option<Vec<ScoreBoardItem>> = None;
    let mut semester_summaries_data: Option<Vec<SemesterSummaryItem>> = None;
    let mut certificates_data: Option<Vec<CertificateItem>> = None;
    let mut has_data = false;

    // Fetch scoreboard and semester summaries for transcript or diploma
    if is_transcript || is_diploma {
        // Get scoreboard data
        let scoreboard = score_repo
            .get_scoreboard_by_user_id(user_id)
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to get scoreboard: {}", e),
                )
            })?;

        if !scoreboard.is_empty() {
            has_data = true;
            scoreboard_data = Some(
                scoreboard
                    .into_iter()
                    .map(|s| ScoreBoardItem {
                        course_id: s.course_id,
                        course_name: s.course_name,
                        course_code: s.course_code,
                        credits: s.credits,
                        score1: s
                            .score1
                            .map(|d| d.to_string().parse::<f64>().unwrap_or(0.0)),
                        score2: s
                            .score2
                            .map(|d| d.to_string().parse::<f64>().unwrap_or(0.0)),
                        score3: s
                            .score3
                            .map(|d| d.to_string().parse::<f64>().unwrap_or(0.0)),
                        score4: s
                            .score4
                            .map(|d| d.to_string().parse::<f64>().unwrap_or(0.0)),
                        score5: s
                            .score5
                            .map(|d| d.to_string().parse::<f64>().unwrap_or(0.0)),
                        score6: s
                            .score6
                            .map(|d| d.to_string().parse::<f64>().unwrap_or(0.0)),
                        letter_grade: s.letter_grade,
                        status: s.status,
                        semester: s.semester,
                        academic_year: s.academic_year,
                        metadata: s.metadata,
                    })
                    .collect(),
            );
        }

        // Get semester summaries
        let summaries = score_repo
            .get_semester_summaries_by_user_id(user_id)
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to get semester summaries: {}", e),
                )
            })?;

        if !summaries.is_empty() {
            has_data = true;
            semester_summaries_data = Some(
                summaries
                    .into_iter()
                    .map(|s| SemesterSummaryItem {
                        semester: s.semester,
                        academic_year: s.academic_year,
                        gpa: s.gpa.to_string().parse::<f64>().unwrap_or(0.0),
                        classification: s.classification,
                        total_credits: s.total_credits,
                        total_passed_credits: s.total_passed_credits,
                        metadata: s.metadata,
                    })
                    .collect(),
            );
        }
    }

    // Fetch certificates for certificate or diploma
    if is_certificate || is_diploma {
        let certificates = if is_diploma {
            // For diploma, get graduation certificate
            score_repo
                .get_certificates_by_user_id_and_type(user_id, "Bằng tốt nghiệp")
                .await
                .map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Failed to get certificates: {}", e),
                    )
                })?
        } else {
            // For certificate, get all certificates or specific type
            score_repo
                .get_certificates_by_user_id(user_id)
                .await
                .map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Failed to get certificates: {}", e),
                    )
                })?
        };

        if !certificates.is_empty() {
            has_data = true;
            certificates_data = Some(
                certificates
                    .into_iter()
                    .map(|(c, doc_type)| CertificateItem {
                        certificate_name: c.certificate_name.clone(),
                        document_type_name: doc_type
                            .as_ref()
                            .map(|dt| dt.document_type_name.clone())
                            .unwrap_or_else(|| "Unknown".to_string()),
                        issued_date: c.issued_date.to_string(),
                        expiry_date: c.expiry_date.map(|d| d.to_string()),
                        description: c.description,
                        metadata: c.metadata,
                    })
                    .collect(),
            );
        }
    }

    if !has_data {
        return Ok((
            StatusCode::OK,
            Json(DocumentDataResponse {
                has_data: false,
                message: format!(
                    "Không có dữ liệu cho loại tài liệu: {}",
                    payload.document_type_name
                ),
                data: Some(DocumentData {
                    user: Some(user_info),
                    scoreboard: None,
                    semester_summaries: None,
                    certificates: None,
                }),
            }),
        ));
    }

    Ok((
        StatusCode::OK,
        Json(DocumentDataResponse {
            has_data: true,
            message: "Dữ liệu đã được lấy thành công".to_string(),
            data: Some(DocumentData {
                user: Some(user_info),
                scoreboard: scoreboard_data,
                semester_summaries: semester_summaries_data,
                certificates: certificates_data,
            }),
        }),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/documents/mock/certificate",
    request_body = MockCertificateRequest,
    responses(
        (status = 200, description = "Certificate created successfully", body = MockCertificateResponse),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Documents"
)]
pub async fn mock_certificate(
    AuthClaims(_auth_claims): AuthClaims,
    Json(payload): Json<MockCertificateRequest>,
) -> Result<(StatusCode, Json<MockCertificateResponse>), (StatusCode, String)> {
    // Find user by email
    let user_repo = UserRepository::new();
    let user = user_repo
        .find_by_email(&payload.user_email)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to find user: {}", e),
            )
        })?;

    let user_id = user.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            format!("User with email {} not found", payload.user_email),
        )
    })?;

    let user_id = user_id.user_id;

    // Parse dates
    let issued_date = NaiveDate::parse_from_str(&payload.issued_date, "%Y-%m-%d").map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("Invalid issued_date format. Expected YYYY-MM-DD: {}", e),
        )
    })?;

    let expiry_date = if let Some(expiry_date_str) = &payload.expiry_date {
        Some(
            NaiveDate::parse_from_str(expiry_date_str, "%Y-%m-%d").map_err(|e| {
                (
                    StatusCode::BAD_REQUEST,
                    format!("Invalid expiry_date format. Expected YYYY-MM-DD: {}", e),
                )
            })?,
        )
    } else {
        None
    };

    let db = DATABASE_CONNECTION
        .get()
        .expect("DATABASE_CONNECTION not set");
    let document_type = document_type::Entity::find_by_id(payload.document_type_id)
        .one(db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get document_type: {}", e),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!(
                    "Document type with id {} not found",
                    payload.document_type_id
                ),
            )
        })?;

    let score_repo = ScoreRepository::new();
    let certificate = score_repo
        .create_certificate_with_data(
            user_id,
            document_type.document_type_id,
            &payload.certificate_name,
            issued_date,
            expiry_date,
            payload.description.as_deref(),
            payload.metadata.clone(),
        )
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to create certificate: {}", e),
            )
        })?;

    Ok((
        StatusCode::OK,
        Json(MockCertificateResponse {
            success: true,
            message: "Certificate created successfully".to_string(),
            certificate: Some(CertificateItem {
                certificate_name: certificate.certificate_name,
                document_type_name: document_type.document_type_name,
                issued_date: certificate.issued_date.to_string(),
                expiry_date: certificate.expiry_date.map(|d| d.to_string()),
                description: certificate.description,
                metadata: certificate.metadata,
            }),
        }),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/documents/mock/transcript",
    request_body = MockTranscriptRequest,
    responses(
        (status = 200, description = "Transcript data created successfully", body = MockTranscriptResponse),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Documents"
)]
pub async fn mock_transcript(
    AuthClaims(_auth_claims): AuthClaims,
    Json(payload): Json<MockTranscriptRequest>,
) -> Result<(StatusCode, Json<MockTranscriptResponse>), (StatusCode, String)> {
    // Find user by email
    let user_repo = UserRepository::new();
    let user = user_repo
        .find_by_email(&payload.user_email)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to find user: {}", e),
            )
        })?;

    let user = user.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            format!("User with email {} not found", payload.user_email),
        )
    })?;

    let user_id = user.user_id;

    let score_repo = ScoreRepository::new();
    let (scoreboard_records, semester_summaries) = score_repo
        .create_mock_transcript_4_semesters(user_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to create transcript data: {}", e),
            )
        })?;

    Ok((
        StatusCode::OK,
        Json(MockTranscriptResponse {
            success: true,
            message: "Transcript data created successfully with 4 semesters".to_string(),
            scoreboard_count: scoreboard_records.len() as u32,
            semester_summaries_count: semester_summaries.len() as u32,
        }),
    ))
}

