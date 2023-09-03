use std::{
    env,
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use api::{Version, VersionManifest};
use launch::LaunchData;
use reqwest::Client;
use tokio::fs;
use tracing::{error, info, Level};

mod api;
mod download;
mod launch;

async fn launch_version(working_dir: &Path, version: &Version) -> anyhow::Result<Arc<AtomicBool>> {
    let stop_signal = Arc::new(AtomicBool::new(false));
    let launch_data = LaunchData {
        use_aikar: true,
        working_dir: working_dir.to_path_buf(),
        version: version.clone(),
        stop_signal: stop_signal.clone(),
    };

    tokio::task::spawn(async move {
        let res = launch_data.start(&Client::new()).await;

        if let Err(e) = res {
            error!("Error in run version task {:?}", e);
        }
    });

    Ok(stop_signal)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing::subscriber::set_global_default(
        tracing_subscriber::fmt()
            .with_max_level(Level::INFO)
            .finish(),
    )?;

    let client = Client::new();
    let working_dir_str =
        env::var("BLEEDINGEDGE_WORKING_DIRECTORY").unwrap_or_else(|_| "run".to_owned());
    let working_dir = Path::new(&working_dir_str);
    if !working_dir.exists() {
        fs::create_dir_all(working_dir).await?;
    }
    let mut version = VersionManifest::fetch(&client)
        .await?
        .absolute_latest()
        .await?
        .unwrap();
    let mut current_stop_signal = launch_version(working_dir, &version).await?;
    let mut interval = tokio::time::interval(Duration::from_secs(3600));

    loop {
        interval.tick().await;
        info!("Checking latest version!");
        let latest_version = VersionManifest::fetch(&client)
            .await?
            .absolute_latest()
            .await?
            .unwrap();

        if latest_version.id == version.id {
            info!("Current version and latest version ID match, skipping!");
            continue;
        }

        current_stop_signal.store(true, Ordering::Release);
        version = latest_version;
        current_stop_signal = launch_version(working_dir, &version).await?;
    }
}
