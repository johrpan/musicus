use crate::{Client, Result};
use log::info;
use musicus_database::Instrument;

impl Client {
    /// Get all available instruments from the server.
    pub async fn get_instruments(&self) -> Result<Vec<Instrument>> {
        info!("Get instruments");
        let body = self.get("instruments").await?;
        let instruments: Vec<Instrument> = serde_json::from_str(&body)?;
        Ok(instruments)
    }

    /// Post a new instrument to the server.
    pub async fn post_instrument(&self, data: &Instrument) -> Result<()> {
        info!("Post instrument {:?}", data);
        self.post("instruments", serde_json::to_string(data)?)
            .await?;
        Ok(())
    }
}
