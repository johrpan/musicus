use crate::backend::{Backend, Instrument, Result};

impl Backend {
    /// Get all available instruments from the server.
    pub async fn get_instruments(&self) -> Result<Vec<Instrument>> {
        let body = self.get("instruments").await?;
        let instruments: Vec<Instrument> = serde_json::from_str(&body)?;
        Ok(instruments)
    }

    /// Post a new instrument to the server.
    pub async fn post_instrument(&self, data: &Instrument) -> Result<()> {
        self.post("instruments", serde_json::to_string(data)?).await?;
        Ok(())
    }
}
