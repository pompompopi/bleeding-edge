use std::{
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use api::{Version, VersionManifest};
use environment::env_var_else;
use launch::LaunchData;
use reqwest::Client;
use tokio::fs;
use tracing::{error, info, Level};

mod api;
mod archive;
mod download;
mod environment;
mod launch;

async fn launch_version(
    working_dir: &Path,
    version: &Version,
) -> anyhow::Result<(Arc<AtomicBool>, Arc<AtomicBool>)> {
    let stop_signal = Arc::new(AtomicBool::new(false));
    let stopped_signal = Arc::new(AtomicBool::new(false));
    let launch_data = LaunchData {
        use_aikar: env_var_else("BLEEDINGEDGE_USE_AIKAR", "1") == "1",
        working_dir: working_dir.to_path_buf(),
        version: version.clone(),
        stop_signal: stop_signal.clone(),
        stopped_signal: stopped_signal.clone(),
    };

    tokio::task::spawn(async move {
        let res = launch_data.start(&Client::new()).await;

        if let Err(e) = res {
            error!("Error in run version task {:?}", e);
        }
    });

    Ok((stop_signal, stopped_signal))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing::subscriber::set_global_default(
        tracing_subscriber::fmt()
            .with_max_level(Level::INFO)
            .finish(),
    )?;

    let client = Client::new();
    let working_dir_string = env_var_else("BLEEDINGEDGE_WORKING_DIRECTORY", "run");
    let working_dir = Path::new(&working_dir_string);
    if !working_dir.exists() {
        fs::create_dir_all(working_dir).await?;
    }

    let backup_dir_string = env_var_else("BLEEDINGEDGE_BACKUP_DIRECTORY", "backups");
    let backup_dir = Path::new(&backup_dir_string);
    if !backup_dir.exists() {
        fs::create_dir_all(backup_dir).await?;
    }

    let mut version = VersionManifest::fetch(&client)
        .await?
        .absolute_latest()
        .await?
        .unwrap();
    let mut signals = launch_version(working_dir, &version).await?;
    let mut interval = tokio::time::interval(Duration::from_secs(3600));
    let mut stopped_signal_query_interval = tokio::time::interval(Duration::from_secs(1));

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

        signals.0.store(true, Ordering::Release);

        while !signals.1.load(Ordering::Acquire) {
            stopped_signal_query_interval.tick().await;
        }

        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
        let mut archive_name = String::from("backup-");
        archive_name.push_str(&now.to_string());
        archive_name.push_str("-");
        archive_name.push_str(&version.id);

        archive::archive(working_dir, backup_dir, &archive_name)?;
        version = latest_version;
        signals = launch_version(working_dir, &version).await?;
    }
}
