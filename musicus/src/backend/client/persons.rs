use super::Backend;
use crate::database::Person;
use anyhow::Result;

impl Backend {
    /// Get all available persons from the server.
    pub async fn get_persons(&self) -> Result<Vec<Person>> {
        let body = self.get("persons").await?;
        let persons: Vec<Person> = serde_json::from_str(&body)?;
        Ok(persons)
    }

    /// Post a new person to the server.
    pub async fn post_person(&self, data: &Person) -> Result<()> {
        self.post("persons", serde_json::to_string(data)?).await?;
        Ok(())
    }
}
