//! Downloading to local files.
//!
//! The library does not download anything; the application downloads to a
//! temporary local file and hands that file to the library's synchronous
//! import/install API. A download only becomes usable once it is whole, so an
//! interrupted download can never be mistaken for a finished one.

use std::path::Path;

use anyhow::Result;
use futures_util::StreamExt;
use tempfile::NamedTempFile;
use tokio::io::AsyncWriteExt;

use crate::process::ExchangeBridge;

/// How long a download may stall before it is considered failed.
///
/// There is deliberately no overall request timeout: a library archive can
/// legitimately take a long time to transfer. This bounds the time without any
/// progress instead, which is what actually distinguishes a stalled server from
/// a slow one.
const DOWNLOAD_READ_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);

/// Download `url` into a new temporary file, in `dir` if given, reporting
/// progress and cancellation through `bridge`.
///
/// The file is complete when this returns; the caller moves it into place (or
/// hands it to the library, which moves its own copy).
pub fn download(url: &str, dir: Option<&Path>, bridge: &ExchangeBridge) -> Result<NamedTempFile> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    runtime.block_on(download_async(url, dir, bridge))
}

async fn download_async<'a>(
    url: &str,
    dir: Option<&Path>,
    bridge: &ExchangeBridge<'a>,
) -> Result<NamedTempFile> {
    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(10))
        .read_timeout(DOWNLOAD_READ_TIMEOUT)
        .build()?;

    let response = client.get(url).send().await?;
    response.error_for_status_ref()?;

    let total_size = response.content_length();
    let mut body_stream = response.bytes_stream();

    let file = match dir {
        Some(dir) => NamedTempFile::new_in(dir)?,
        None => NamedTempFile::new()?,
    };

    let mut writer =
        tokio::io::BufWriter::new(tokio::fs::File::from_std(file.as_file().try_clone()?));

    let mut downloaded: u64 = 0;
    while let Some(chunk) = body_stream.next().await {
        bridge.check()?;

        let chunk: Vec<u8> = chunk?.into();
        let chunk_size = chunk.len() as u64;

        writer.write_all(&chunk).await?;

        if let Some(total_size) = total_size {
            downloaded += chunk_size;
            bridge.progress(downloaded as f64 / total_size as f64)?;
        }
    }

    writer.flush().await?;

    Ok(file)
}
