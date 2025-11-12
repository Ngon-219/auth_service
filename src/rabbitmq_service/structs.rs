use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct RegisterNewUserMessage {
    pub private_key: String,
    pub wallet_address: String,
    pub student_code: String,
    pub full_name: String,
    pub email: String,
}
