use isahc::{AsyncBody, Request, Response};
use isahc::http::StatusCode;
use isahc::prelude::*;
use serde::Serialize;
use std::time::Duration;
use std::cell::RefCell;

pub mod ensembles;
pub use ensembles::*;

pub mod error;
pub use error::*;

pub mod instruments;
pub use instruments::*;

pub mod mediums;
pub use mediums::*;

pub mod persons;
pub use persons::*;

pub mod recordings;
pub use recordings::*;

pub mod register;
pub use register::*;

pub mod works;
pub use works::*;

/// Credentials used for login.
#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LoginData {
    pub username: String,
    pub password: String,
}

/// A client for accessing the Wolfgang API.
pub struct Client {
    server_url: RefCell<Option<String>>,
    login_data: RefCell<Option<LoginData>>,
    token: RefCell<Option<String>>,
}

impl Client {
    /// Create a new client.
    pub fn new() -> Self {
        Self {
            server_url: RefCell::new(None),
            login_data: RefCell::new(None),
            token: RefCell::new(None),
        }
    }

    /// Set the URL of the Musicus server to connect to.
    pub fn set_server_url(&self, url: &str) {
        self.server_url.replace(Some(url.to_owned()));
    }

    /// Get the currently set server URL.
    pub fn get_server_url(&self) -> Option<String> {
        self.server_url.borrow().clone()
    }

    /// Set the user credentials to use.
    pub fn set_login_data(&self, data: Option<LoginData>) {
        self.login_data.replace(data);
        self.token.replace(None);
    }

    /// Get the currently stored login credentials.
    pub fn get_login_data(&self) -> Option<LoginData> {
        self.login_data.borrow().clone()
    }

    /// Try to login a user with the provided credentials and return, wether the login suceeded.
    pub async fn login(&self) -> Result<bool> {
        let server_url = self.server_url()?;
        let data = self.login_data()?;

        let request = Request::post(format!("{}/login", server_url))
            .timeout(Duration::from_secs(10))
            .header("Content-Type", "application/json")
            .body(serde_json::to_string(&data)?)?;

        let mut response = isahc::send_async(request).await?;

        let success = match response.status() {
            StatusCode::OK => {
                let token = response.text().await?;
                self.token.replace(Some(token));
                true
            }
            StatusCode::UNAUTHORIZED => false,
            status_code => Err(Error::UnexpectedResponse(status_code))?,
        };

        Ok(success)
    }

    /// Make an unauthenticated get request to the server.
    async fn get(&self, url: &str) -> Result<String> {
        let server_url = self.server_url()?;

        let mut response = Request::get(format!("{}/{}", server_url, url))
            .timeout(Duration::from_secs(10))
            .body(())?
            .send_async()
            .await?;

        let body = response.text().await?;

        Ok(body)
    }

    /// Make an authenticated post request to the server.
    async fn post(&self, url: &str, body: String) -> Result<String> {
        let body = if self.token.borrow().is_some() {
            let mut response = self.post_priv(url, body.clone()).await?;

            // Try one more time (maybe the token was expired)
            if response.status() == StatusCode::UNAUTHORIZED {
                if self.login().await? {
                    response = self.post_priv(url, body).await?;
                } else {
                    Err(Error::LoginFailed)?;
                }
            }

            response.text().await?
        } else {
            let mut response = if self.login().await? {
                self.post_priv(url, body).await?
            } else {
                Err(Error::LoginFailed)?
            };

            response.text().await?
        };

        Ok(body)
    }

    /// Post something to the server assuming there is a valid login token.
    async fn post_priv(&self, url: &str, body: String) -> Result<Response<AsyncBody>> {
        let server_url = self.server_url()?;
        let token = self.token()?;

        let response = Request::post(format!("{}/{}", server_url, url))
            .timeout(Duration::from_secs(10))
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .body(body)?
            .send_async()
            .await?;

        Ok(response)
    }

    /// Require the server URL to be set.
    fn server_url(&self) -> Result<String> {
        self.get_server_url().ok_or(Error::Other("The server URL is not available!"))
    }

    /// Require the login data to be set.
    fn login_data(&self) -> Result<LoginData> {
        self.get_login_data().ok_or(Error::Other("The login data is unset!"))
    }

    /// Require a login token to be set.
    fn token(&self) -> Result<String> {
        self.token.borrow().clone().ok_or(Error::Other("No login token found!"))
    }
}
