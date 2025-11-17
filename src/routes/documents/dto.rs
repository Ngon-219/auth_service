use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

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
pub struct DocumentData {
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
    pub certificate_type: String,
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
    pub certificate_type: String,
    pub issued_date: String, // Format: YYYY-MM-DD
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

