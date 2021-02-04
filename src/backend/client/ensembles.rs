use crate::backend::{Backend, Ensemble, Result};

impl Backend {
    /// Get all available ensembles from the server.
    pub async fn get_ensembles(&self) -> Result<Vec<Ensemble>> {
        let body = self.get("ensembles").await?;
        let ensembles: Vec<Ensemble> = serde_json::from_str(&body)?;
        Ok(ensembles)
    }

    /// Post a new ensemble to the server.
    pub async fn post_ensemble(&self, data: &Ensemble) -> Result<()> {
        self.post("ensembles", serde_json::to_string(data)?).await?;
        Ok(())
    }
}
