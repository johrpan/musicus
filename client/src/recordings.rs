use crate::{Client, Result};
use log::info;
use musicus_database::Recording;

impl Client {
    /// Get all available recordings from the server.
    pub async fn get_recordings_for_work(&self, work_id: &str) -> Result<Vec<Recording>> {
        info!("Get recordings for work {}", work_id);
        let body = self.get(&format!("works/{}/recordings", work_id)).await?;
        let recordings: Vec<Recording> = serde_json::from_str(&body)?;
        Ok(recordings)
    }

    /// Post a new recording to the server.
    pub async fn post_recording(&self, data: &Recording) -> Result<()> {
        info!("Post recording {:?}", data);
        self.post("recordings", serde_json::to_string(data)?)
            .await?;
        Ok(())
    }
}
