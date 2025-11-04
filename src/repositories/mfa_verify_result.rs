use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MfaVerifyResult {
    Success,
    Locked {
        locked_until: Option<i64>,
    },
    CodeAlreadyUsed,
    InvalidCode,
    MfaNotEnabled,
}

impl MfaVerifyResult {
    pub fn is_success(&self) -> bool {
        matches!(self, MfaVerifyResult::Success)
    }

    pub fn reason(&self) -> &'static str {
        match self {
            MfaVerifyResult::Success => "MFA code verified successfully",
            MfaVerifyResult::Locked { .. } => "MFA is locked due to too many failed attempts",
            MfaVerifyResult::CodeAlreadyUsed => "MFA code has already been used",
            MfaVerifyResult::InvalidCode => "Invalid MFA code",
            MfaVerifyResult::MfaNotEnabled => "MFA is not enabled for this user",
        }
    }

    pub fn message(&self) -> String {
        match self {
            MfaVerifyResult::Success => "MFA code verified successfully".to_string(),
            MfaVerifyResult::Locked { locked_until } => {
                if let Some(until) = locked_until {
                    format!("MFA is locked until {} (too many failed attempts)", until)
                } else {
                    "MFA is locked due to too many failed attempts".to_string()
                }
            }
            MfaVerifyResult::CodeAlreadyUsed => "MFA code has already been used".to_string(),
            MfaVerifyResult::InvalidCode => "Invalid MFA code".to_string(),
            MfaVerifyResult::MfaNotEnabled => "MFA is not enabled for this user".to_string(),
        }
    }
}

