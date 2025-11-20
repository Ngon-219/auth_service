use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct RegisterNewUserMessage {
    pub private_key: String,
    pub wallet_address: String,
    pub student_code: String,
    pub full_name: String,
    pub email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_upload_history_id: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RegisterNewManagerMessage {
    pub private_key: String,
    pub wallet_address: String,
    pub email: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AssignRoleMessage {
    pub private_key: String,
    pub user_address: String,
    pub role: u8,
    pub email: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RemoveManagerMessage {
    pub private_key: String,
    pub manager_address: String,
    pub email: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DeactivateStudentMessage {
    pub private_key: String,
    pub student_id: u64,
    pub email: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ActivateStudentMessage {
    pub private_key: String,
    pub student_id: u64,
    pub email: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RegisterStudentsBatchMessage {
    pub private_key: String,
    pub wallet_addresses: Vec<String>,
    pub student_codes: Vec<String>,
    pub full_names: Vec<String>,
    pub emails: Vec<String>,
}
