use crate::{Client, Result};
use log::info;
use musicus_database::Medium;

impl Client {
    /// Get all available mediums from the server, that contain the specified
    /// recording.
    pub async fn get_mediums_for_recording(&self, recording_id: &str) -> Result<Vec<Medium>> {
        info!("Get mediums for recording {}", recording_id);
        let body = self.get(&format!("recordings/{}/mediums", recording_id)).await?;
        let mediums: Vec<Medium> = serde_json::from_str(&body)?;
        Ok(mediums)
    }

    /// Get all available mediums from the server, that match the specified
    /// DiscID.
    pub async fn get_mediums_by_discid(&self, discid: &str) -> Result<Vec<Medium>> {
        info!("Get mediums by discid {}", discid);
        let body = self.get(&format!("discids/{}/mediums", discid)).await?;
        let mediums: Vec<Medium> = serde_json::from_str(&body)?;
        Ok(mediums)
    }

    /// Post a new medium to the server.
    pub async fn post_medium(&self, data: &Medium) -> Result<()> {
        info!("Post medium {:?}", data);
        self.post("mediums", serde_json::to_string(data)?).await?;
        Ok(())
    }
}
