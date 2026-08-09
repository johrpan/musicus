use anyhow::Result;

/// A progress update sent from a background library operation.
#[derive(Debug)]
pub enum ProcessMsg {
    Message(String),
    Progress(f64),
    Result(Result<()>),
}
