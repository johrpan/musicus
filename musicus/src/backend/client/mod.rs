use super::secure;
use super::Backend;
use anyhow::{anyhow, bail, Result};
use gio::prelude::*;
use isahc::http::StatusCode;
use isahc::prelude::*;
use serde::Serialize;
use std::time::Duration;

pub mod persons;
pub use persons::*;

pub mod ensembles;
pub use ensembles::*;

/// Credentials used for login.
#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LoginData {
    pub username: String,
    pub password: String,
}

impl Backend {
    /// Initialize the client.
    pub(super) fn init_client(&self) -> Result<()> {
        if let Some(data) = secure::load_login_data()? {
            self.login_data.replace(Some(data));
        }

        if let Some(url) = self.settings.get_string("server-url") {
            if !url.is_empty() {
                self.server_url.replace(Some(url.to_string()));
            }
        }

        Ok(())
    }

    /// Set the URL of the Musicus server to connect to.
    pub fn set_server_url(&self, url: &str) -> Result<()> {
        self.settings.set_string("server-url", url)?;
        self.server_url.replace(Some(url.to_string()));
        Ok(())
    }

    /// Get the currently used login token.
    pub fn get_token(&self) -> Option<String> {
        self.token.borrow().clone()
    }

    /// Set the login token to use. This will be done automatically by the login method.
    pub fn set_token(&self, token: &str) {
        self.token.replace(Some(token.to_string()));
    }

    /// Get the currently set server URL.
    pub fn get_server_url(&self) -> Option<String> {
        self.server_url.borrow().clone()
    }

    /// Get the currently stored login credentials.
    pub fn get_login_data(&self) -> Option<LoginData> {
        self.login_data.borrow().clone()
    }

    /// Set the user credentials to use.
    pub async fn set_login_data(&self, data: LoginData) -> Result<()> {
        secure::store_login_data(data.clone()).await?;
        self.login_data.replace(Some(data));
        self.token.replace(None);
        Ok(())
    }

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
                true
            }
            StatusCode::UNAUTHORIZED => false,
            _ => bail!("Unexpected response status!"),
        };

        Ok(success)
    }

    /// Make an unauthenticated get request to the server.
    async fn get(&self, url: &str) -> Result<String> {
        let server_url = self.get_server_url().ok_or(anyhow!("No server URL set!"))?;

        let mut response = Request::get(format!("{}/{}", server_url, url))
            .timeout(Duration::from_secs(10))
            .body(())?
            .send_async()
            .await?;

        let body = response.text_async().await?;

        Ok(body)
    }

    /// Make an authenticated post request to the server.
    async fn post(&self, url: &str, body: String) -> Result<String> {
        let body = match self.get_token() {
            Some(_) => {
                let mut response = self.post_priv(url, body.clone()).await?;

                // Try one more time (maybe the token was expired)
                if response.status() == StatusCode::UNAUTHORIZED {
                    if self.login().await? {
                        response = self.post_priv(url, body).await?;
                    } else {
                        bail!("Login failed!");
                    }
                }

                response.text_async().await?
            }
            None => {
                let mut response = if self.login().await? {
                    self.post_priv(url, body).await?
                } else {
                    bail!("Login failed!");
                };

                response.text_async().await?
            }
        };

        Ok(body)
    }

    /// Post something to the server assuming there is a valid login token.
    async fn post_priv(&self, url: &str, body: String) -> Result<Response<Body>> {
        let server_url = self.get_server_url().ok_or(anyhow!("No server URL set!"))?;
        let token = self.get_token().ok_or(anyhow!("No login token!"))?;

        let response = Request::post(format!("{}/{}", server_url, url))
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .body(body)?
            .send_async()
            .await?;

        Ok(response)
    }
}
