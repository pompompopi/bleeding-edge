use std::path::PathBuf;

use anyhow::bail;
use reqwest::Client;
use sha1_smol::Sha1;
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt},
};
use tracing::info;

pub struct Artifact {
    pub url: String,
    pub sha1: String,
    pub path: PathBuf,
}

impl Artifact {
    pub async fn download(&self, client: &Client) -> anyhow::Result<()> {
        // TODO: Add progress indicator
        let mut output = File::create(&self.path).await?;
        let mut res = client.get(&self.url).send().await?;

        info!("Downloading {}...", self.url);
        while let Some(chunk) = res.chunk().await? {
            output.write_all(&chunk).await?;
        }
        info!("Downloaded {}! Checking integrity...", self.url);

        if !self.properly_exists().await? {
            bail!(
                "Checksum of downloaded artifact {:?} did not match!",
                self.path.file_name()
            )
        }

        info!("Integrity check successful.");
        Ok(())
    }

    pub async fn properly_exists(&self) -> anyhow::Result<bool> {
        if !self.path.exists() {
            return Ok(false);
        }

        let mut sha1 = Sha1::new();
        let mut file = File::open(&self.path).await?;
        let mut buf = vec![0u8; 1048576];

        loop {
            let read = file.read(&mut buf).await?;

            if read < 1 {
                break;
            }

            sha1.update(&buf[..read]);
        }

        Ok(sha1.hexdigest() == self.sha1)
    }
}
