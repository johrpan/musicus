use super::Backend;
use crate::database::Recording;
use anyhow::Result;

impl Backend {
    /// Get all available recordings from the server.
    pub async fn get_recordings_for_work(&self, work_id: &str) -> Result<Vec<Recording>> {
        let body = self.get(&format!("works/{}/recordings", work_id)).await?;
        let recordings: Vec<Recording> = serde_json::from_str(&body)?;
        Ok(recordings)
    }

    /// Post a new recording to the server.
    pub async fn post_recording(&self, data: &Recording) -> Result<()> {
        self.post("recordings", serde_json::to_string(data)?).await?;
        Ok(())
    }
}
