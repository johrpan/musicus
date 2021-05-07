use isahc::http::StatusCode;
use isahc::prelude::*;
use isahc::{AsyncBody, Request, Response};
use log::info;
use serde::Serialize;
use std::cell::RefCell;
use std::time::Duration;

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
        info!("Login");

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
            status_code => return Err(Error::UnexpectedResponse(status_code)),
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

        match response.status() {
            StatusCode::OK => Ok(response.text().await?),
            status_code => Err(Error::UnexpectedResponse(status_code)),
        }
    }

    /// Make an authenticated post request to the server.
    async fn post(&self, url: &str, body: String) -> Result<String> {
        // Try to do the request using a cached login token.
        if self.token.borrow().is_some() {
            let mut response = self.post_priv(url, body.clone()).await?;

            // If authorization failed, try again below. Else, return early.
            match response.status() {
                StatusCode::UNAUTHORIZED => info!("Token may be expired"),
                StatusCode::OK => return Ok(response.text().await?),
                status_code => return Err(Error::UnexpectedResponse(status_code)),
            }
        }

        if self.login().await? {
            let mut response = self.post_priv(url, body).await?;

            match response.status() {
                StatusCode::OK => Ok(response.text().await?),
                StatusCode::UNAUTHORIZED => Err(Error::Unauthorized),
                status_code => Err(Error::UnexpectedResponse(status_code)),
            }
        } else {
            Err(Error::LoginFailed)
        }
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
        self.get_server_url()
            .ok_or(Error::Other("The server URL is not available!"))
    }

    /// Require the login data to be set.
    fn login_data(&self) -> Result<LoginData> {
        self.get_login_data()
            .ok_or(Error::Other("The login data is unset!"))
    }

    /// Require a login token to be set.
    fn token(&self) -> Result<String> {
        self.token
            .borrow()
            .clone()
            .ok_or(Error::Other("No login token found!"))
    }
}
