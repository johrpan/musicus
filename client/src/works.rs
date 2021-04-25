use crate::{Client, Result};
use log::info;
use musicus_database::Work;

impl Client {
    /// Get all available works from the server.
    pub async fn get_works(&self, composer_id: &str) -> Result<Vec<Work>> {
        info!("Get works by composer {}", composer_id);
        let body = self.get(&format!("persons/{}/works", composer_id)).await?;
        let works: Vec<Work> = serde_json::from_str(&body)?;
        Ok(works)
    }

    /// Post a new work to the server.
    pub async fn post_work(&self, data: &Work) -> Result<()> {
        info!("Post work {:?}", data);
        self.post("works", serde_json::to_string(data)?).await?;
        Ok(())
    }
}
