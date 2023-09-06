use std::path::Path;

use once_cell::sync::Lazy;
use regex::Regex;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::download::Artifact;

#[derive(Serialize, Deserialize, Debug)]
pub struct VersionPackageDownload {
    pub sha1: String,
    pub url: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VersionPackageDownloads {
    pub server: VersionPackageDownload,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VersionPackageMeta {
    pub downloads: VersionPackageDownloads,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VersionManifest {
    pub latest: LatestVersions,
    pub versions: Vec<Version>,
}

impl VersionManifest {
    pub async fn fetch(client: &Client) -> anyhow::Result<VersionManifest> {
        Ok(client
            .get("https://piston-meta.mojang.com/mc/game/version_manifest_v2.json")
            .send()
            .await?
            .json()
            .await?)
    }

    pub async fn absolute_latest(&self) -> anyhow::Result<Option<Version>> {
        static REGEX: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r"((1\.\d+(\.\d{1,2})?(-(rc|pre)\d{1,2})?)|\d\dw\d\d[a-z])").unwrap()
        });
        Ok(self
            .versions
            .iter()
            .find(|v| REGEX.is_match(&v.id))
            .map(|v| v.clone()))
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LatestVersions {
    pub release: String,
    pub snapshot: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Version {
    pub id: String,
    #[serde(rename = "type")]
    pub version_type: String,
    pub url: String,
}

impl Version {
    pub async fn as_artifact(&self, client: &Client, path: &Path) -> anyhow::Result<Artifact> {
        let server_downloads = client
            .get(&self.url)
            .send()
            .await?
            .json::<VersionPackageMeta>()
            .await?
            .downloads
            .server;

        Ok(Artifact {
            sha1: server_downloads.sha1,
            url: server_downloads.url,
            path: path.to_path_buf(),
        })
    }
}
