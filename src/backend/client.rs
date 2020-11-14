use super::Backend;
use anyhow::{anyhow, bail, Result};
use isahc::http::StatusCode;
use isahc::prelude::*;

impl Backend {
    /// Try to login a user with the provided credentials and return, wether the login suceeded.
    pub async fn login(&self) -> Result<bool> {
        let server_url = self.get_server_url().ok_or(anyhow!("No server URL set!"))?;
        let data = self.get_login_data().ok_or(anyhow!("No login data set!"))?;

        let request = Request::post(format!("{}/login", server_url))
            .header("Content-Type", "application/json")
            .body(serde_json::to_string(&data)?)?;

        let mut response = isahc::send_async(request).await?;

        let success = match response.status() {
            StatusCode::OK => {
                let token = response.text_async().await?;
                self.set_token(&token);
                println!("{}", &token);
                true
            }
            StatusCode::UNAUTHORIZED => false,
            _ => bail!("Unexpected response status!"),
        };

        Ok(success)
    }
}
