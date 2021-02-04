use crate::{Client, Result};
use isahc::http::StatusCode;
use isahc::prelude::*;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Response body data for captcha requests.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Captcha {
    pub id: String,
    pub question: String,
}

/// Request body data for user registration.
#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserRegistration {
    pub username: String,
    pub password: String,
    pub email: Option<String>,
    pub captcha_id: String,
    pub answer: String,
}

impl Client {
    /// Request a new captcha for registration.
    pub async fn get_captcha(&self) -> Result<Captcha> {
        let body = self.get("captcha").await?;
        let captcha = serde_json::from_str(&body)?;
        Ok(captcha)
    }

    /// Register a new user and return whether the process suceeded. This will
    /// not store the new login credentials.
    pub async fn register(&self, data: UserRegistration) -> Result<bool> {
        let server_url = self.server_url()?;

        let response = Request::post(format!("{}/users", server_url))
            .timeout(Duration::from_secs(10))
            .header("Content-Type", "application/json")
            .body(serde_json::to_string(&data)?)?
            .send_async()
            .await?;

        Ok(response.status() == StatusCode::OK)
    }
}
