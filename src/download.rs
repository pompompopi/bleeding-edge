use std::path::PathBuf;

use anyhow::bail;
use indicatif::{ProgressBar, ProgressStyle};
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

fn create_bar(len: u64) -> anyhow::Result<ProgressBar> {
    let progress_bar = ProgressBar::new(len);
    let style = ProgressStyle::with_template(
        "{msg} - [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes}",
    )?;
    progress_bar.set_style(style);
    Ok(progress_bar)
}

impl Artifact {
    pub async fn download(&self, client: &Client) -> anyhow::Result<()> {
        let mut output = File::create(&self.path).await?;
        let mut res = client.get(&self.url).send().await?;

        let size = res.content_length().unwrap();
        let progress_bar = create_bar(size)?;
        progress_bar.set_message(format!("downloading {}", self.url));
        while let Some(chunk) = res.chunk().await? {
            output.write_all(&chunk).await?;
            progress_bar.set_position((progress_bar.position() + chunk.len() as u64).min(size));
        }
        progress_bar.finish_and_clear();
        info!("Artifact {} downloaded", self.url);

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

        let len = file.metadata().await?.len();
        let progress_bar = create_bar(len)?;
        progress_bar.set_message(format!("hashing {:?}", self.path));

        loop {
            let read = file.read(&mut buf).await?;

            if read < 1 {
                break;
            }

            sha1.update(&buf[..read]);
            progress_bar.set_position((progress_bar.position() + read as u64).min(len));
        }

        let hash = sha1.hexdigest();
        progress_bar.finish_and_clear();
        info!(
            "Completed hashing {:?}, hash is {}",
            self.path.clone(),
            hash
        );
        Ok(hash == self.sha1)
    }
}
