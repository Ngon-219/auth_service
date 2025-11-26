use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DocumentDataRequest {
    pub document_type_name: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DocumentDataResponse {
    pub has_data: bool,
    pub message: String,
    pub data: Option<DocumentData>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UserInfo {
    pub user_id: String,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub student_code: Option<String>,
    pub role: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DocumentData {
    pub user: Option<UserInfo>,
    pub scoreboard: Option<Vec<ScoreBoardItem>>,
    pub semester_summaries: Option<Vec<SemesterSummaryItem>>,
    pub certificates: Option<Vec<CertificateItem>>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ScoreBoardItem {
    pub course_id: String,
    pub course_name: String,
    pub course_code: Option<String>,
    pub credits: i32,
    pub score1: Option<f64>,
    pub score2: Option<f64>,
    pub score3: Option<f64>,
    pub score4: Option<f64>,
    pub score5: Option<f64>,
    pub score6: Option<f64>,
    pub letter_grade: Option<String>,
    pub status: Option<String>,
    pub semester: String,
    pub academic_year: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SemesterSummaryItem {
    pub semester: String,
    pub academic_year: String,
    pub gpa: f64,
    pub classification: Option<String>,
    pub total_credits: Option<i32>,
    pub total_passed_credits: Option<i32>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CertificateItem {
    pub document_type_name: String,
    pub issued_date: String,
    pub expiry_date: Option<String>,
    pub description: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MockDataRequest {
    pub email: String,
    // pub document_type: String, // "Certificate", "Diploma", "Transcript"
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MockDataResponse {
    pub success: bool,
    pub message: String,
    pub data_created: Option<MockDataCreated>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MockDataCreated {
    pub certificates_count: Option<u32>,
    pub scoreboard_count: Option<u32>,
    pub semester_summaries_count: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MockCertificateRequest {
    pub user_email: String,
    pub document_type_id: Uuid,
    pub issued_date: String,         // Format: YYYY-MM-DD
    pub expiry_date: Option<String>, // Format: YYYY-MM-DD
    pub description: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MockCertificateResponse {
    pub success: bool,
    pub message: String,
    pub certificate: Option<CertificateItem>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MockTranscriptRequest {
    pub user_email: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MockTranscriptResponse {
    pub success: bool,
    pub message: String,
    pub scoreboard_count: u32,
    pub semester_summaries_count: u32,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DocumentTypeResponse {
    pub document_type_id: Uuid,
    pub document_type_name: String,
    pub description: Option<String>,
    pub template_pdf: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub created_by: Option<Uuid>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateDocumentTypeRequest {
    #[schema(example = "Certificate")]
    pub document_type_name: Option<String>,
    #[schema(example = "Chứng chỉ")]
    pub description: Option<String>,
    #[schema(example = "template.pdf")]
    pub template_pdf: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateDocumentTypeResponse {
    pub document_type_id: Uuid,
    pub message: String,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateDocumentRequest {
    #[schema(example = "pending")]
    pub status: Option<String>,
    #[schema(example = true)]
    pub is_valid: Option<bool>,
    #[schema(example = "QmHash...")]
    pub ipfs_hash: Option<String>,
    #[schema(example = "QmPdfHash...")]
    pub pdf_ipfs_hash: Option<String>,
    #[schema(example = "0x123...")]
    pub document_hash: Option<String>,
    #[schema(example = "0xabc...")]
    pub tx_hash: Option<String>,
    #[schema(example = "0xdef...")]
    pub blockchain_doc_id: Option<String>,
    #[schema(example = 123)]
    pub token_id: Option<i64>,
    pub metadata: Option<serde_json::Value>,
    pub pdf_schema: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateDocumentResponse {
    pub document_id: Uuid,
    pub message: String,
}
