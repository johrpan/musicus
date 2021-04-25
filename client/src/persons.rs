use crate::{Client, Result};
use log::info;
use musicus_database::Person;

impl Client {
    /// Get all available persons from the server.
    pub async fn get_persons(&self) -> Result<Vec<Person>> {
        info!("Get persons");
        let body = self.get("persons").await?;
        let persons: Vec<Person> = serde_json::from_str(&body)?;
        Ok(persons)
    }

    /// Post a new person to the server.
    pub async fn post_person(&self, data: &Person) -> Result<()> {
        info!("Post person {:?}", data);
        self.post("persons", serde_json::to_string(data)?).await?;
        Ok(())
    }
}
