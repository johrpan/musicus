use super::Backend;
use crate::database::Person;
use anyhow::{anyhow, Result};
use isahc::prelude::*;
use std::time::Duration;

impl Backend {
    /// Get all available persons from the server.
    pub async fn get_persons(&self) -> Result<Vec<Person>> {
        let server_url = self.get_server_url().ok_or(anyhow!("No server URL set!"))?;

        let mut response = Request::get(format!("{}/persons", server_url))
            .timeout(Duration::from_secs(10))
            .body(())?
            .send_async()
            .await?;

        let body = response.text_async().await?;

        let persons: Vec<Person> = serde_json::from_str(&body)?;

        Ok(persons)
    }
}
