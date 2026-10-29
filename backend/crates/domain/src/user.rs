use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A user account.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub channel_name: String,
    pub avatar_url: Option<String>,
    pub bio: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Request body for POST /api/auth/register.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub name: String,
    pub channel_name: String,
}

impl RegisterRequest {
    pub fn validate(&self) -> Result<(), String> {
        if self.email.is_empty() {
            return Err("email required".into());
        }
        if self.password.len() < 8 {
            return Err("password must be at least 8 characters".into());
        }
        if self.name.is_empty() {
            return Err("name required".into());
        }
        if self.channel_name.is_empty() {
            return Err("channelName required".into());
        }
        Ok(())
    }
}

/// Request body for POST /api/auth/login.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

/// Token response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenResponse {
    pub token: String,
    pub user: User,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_request_validates() {
        let ok = RegisterRequest {
            email: "a@b.com".into(),
            password: "password123".into(),
            name: "A".into(),
            channel_name: "chan".into(),
        };
        assert!(ok.validate().is_ok());

        let short_pw = RegisterRequest {
            password: "x".into(),
            ..ok.clone()
        };
        assert!(short_pw.validate().is_err());

        let empty_name = RegisterRequest {
            name: "".into(),
            ..ok.clone()
        };
        assert!(empty_name.validate().is_err());
    }

    #[test]
    fn user_serializes_camel_case() {
        let u = User {
            id: Uuid::new_v4(),
            email: "a@b.com".into(),
            name: "A".into(),
            channel_name: "chan".into(),
            avatar_url: None,
            bio: None,
            created_at: Utc::now(),
        };
        let s = serde_json::to_string(&u).unwrap();
        assert!(s.contains("\"channelName\""));
    }
}
