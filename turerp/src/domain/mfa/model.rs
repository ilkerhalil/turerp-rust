//! MFA domain model

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// MFA method enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum MfaMethod {
    Totp,
    Sms,
    #[default]
    None,
}

impl std::fmt::Display for MfaMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MfaMethod::Totp => write!(f, "totp"),
            MfaMethod::Sms => write!(f, "sms"),
            MfaMethod::None => write!(f, "none"),
        }
    }
}

impl std::str::FromStr for MfaMethod {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "totp" => Ok(MfaMethod::Totp),
            "sms" => Ok(MfaMethod::Sms),
            "none" => Ok(MfaMethod::None),
            _ => Err(format!("Invalid MFA method: {}", s)),
        }
    }
}

/// MFA settings for a user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MfaSettings {
    pub user_id: i64,
    pub tenant_id: i64,
    pub totp_secret: Option<String>,
    pub mfa_enabled: bool,
    pub backup_codes: Vec<String>,
    pub method: MfaMethod,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

/// MFA challenge for verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MfaChallenge {
    pub user_id: i64,
    pub tenant_id: i64,
    pub code: String,
    pub expires_at: DateTime<Utc>,
    pub attempts: i32,
    pub created_at: DateTime<Utc>,
}

/// Request to verify TOTP during setup
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct VerifyTotpRequest {
    pub code: String,
}

/// Request to enable MFA
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct EnableMfaRequest {
    pub method: MfaMethod,
}

/// Request to disable MFA
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct DisableMfaRequest {
    pub password: String,
}

/// Response with MFA status
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MfaStatusResponse {
    pub user_id: i64,
    pub mfa_enabled: bool,
    pub method: MfaMethod,
}

/// Response when MFA setup is initiated
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MfaSetupResponse {
    pub qr_code_uri: String,
    pub secret: String,
}

/// Response after backup codes are generated
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BackupCodesResponse {
    pub backup_codes: Vec<String>,
}

/// Request to verify MFA code during login
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct VerifyMfaRequest {
    pub mfa_token: String,
    pub code: String,
}

/// Response when MFA is required during login
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MfaRequiredResponse {
    pub mfa_token: String,
    pub message: String,
}

impl From<MfaSettings> for MfaStatusResponse {
    fn from(settings: MfaSettings) -> Self {
        Self {
            user_id: settings.user_id,
            mfa_enabled: settings.mfa_enabled,
            method: settings.method,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mfa_method_display() {
        assert_eq!(MfaMethod::Totp.to_string(), "totp");
        assert_eq!(MfaMethod::Sms.to_string(), "sms");
        assert_eq!(MfaMethod::None.to_string(), "none");
    }

    #[test]
    fn test_mfa_method_from_str() {
        assert_eq!("totp".parse::<MfaMethod>().unwrap(), MfaMethod::Totp);
        assert_eq!("sms".parse::<MfaMethod>().unwrap(), MfaMethod::Sms);
        assert_eq!("none".parse::<MfaMethod>().unwrap(), MfaMethod::None);
        assert!("invalid".parse::<MfaMethod>().is_err());
    }

    #[test]
    fn test_mfa_status_response_from_settings() {
        let settings = MfaSettings {
            user_id: 1,
            tenant_id: 1,
            totp_secret: None,
            mfa_enabled: true,
            backup_codes: vec![],
            method: MfaMethod::Totp,
            created_at: Utc::now(),
            updated_at: None,
        };

        let response: MfaStatusResponse = settings.into();
        assert_eq!(response.user_id, 1);
        assert!(response.mfa_enabled);
        assert_eq!(response.method, MfaMethod::Totp);
    }
}
