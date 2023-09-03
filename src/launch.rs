use std::{
    env,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use reqwest::Client;
use tokio::{fs::File, io::AsyncWriteExt, process};
use tracing::info;

use crate::api::Version;

pub struct LaunchData {
    pub use_aikar: bool,
    pub working_dir: PathBuf,
    pub version: Version,
    pub stop_signal: Arc<AtomicBool>,
}

impl LaunchData {
    pub async fn start(&self, client: &Client) -> anyhow::Result<()> {
        let path = self.working_dir.canonicalize()?;
        let server_jar_path = path.join("server.jar");

        // Download the server if needed
        let artifact = self.version.as_artifact(client, &server_jar_path).await?;

        if !artifact.properly_exists().await? {
            artifact.download(client).await?;
        }

        // Always agree to the eula on start
        let eula_path = self.working_dir.join("eula.txt");
        File::create(eula_path)
            .await?
            .write_all("eula=true".as_bytes())
            .await?;

        let mut jvm_args = vec![
            "-Xms".to_owned()
                + &env::var("BLEEDINGEDGE_MIN_MEM").unwrap_or_else(|_| "1g".to_owned()),
            "-Xmx".to_owned()
                + &env::var("BLEEDINGEDGE_MAX_MEM").unwrap_or_else(|_| "1g".to_owned()),
        ];

        if self.use_aikar {
            // kill me
            jvm_args.append(
                &mut [
                    "-XX:+UseG1GC",
                    "-XX:+ParallelRefProcEnabled",
                    "-XX:MaxGCPauseMillis=200",
                    "-XX:+UnlockExperimentalVMOptions",
                    "-XX:+DisableExplicitGC",
                    "-XX:+AlwaysPreTouch",
                    "-XX:G1NewSizePercent=30",
                    "-XX:G1MaxNewSizePercent=40",
                    "-XX:G1HeapRegionSize=8M",
                    "-XX:G1ReservePercent=20",
                    "-XX:G1HeapWastePercent=5",
                    "-XX:G1MixedGCCountTarget=4",
                    "-XX:InitiatingHeapOccupancyPercent=15",
                    "-XX:G1MixedGCLiveThresholdPercent=90",
                    "-XX:G1RSetUpdatingPauseTimePercent=5",
                    "-XX:SurvivorRatio=32",
                    "-XX:+PerfDisableSharedMem",
                    "-XX:MaxTenuringThreshold=1",
                ]
                .into_iter()
                .map(|s| s.to_owned())
                .collect::<Vec<String>>(),
            );
        }

        let mut command = process::Command::new("java");
        command
            .current_dir(path)
            .args(&jvm_args)
            .arg("-jar")
            .arg(server_jar_path)
            .arg("--nogui");

        loop {
            info!("Launching {:?}!", command);
            let mut child = command.spawn()?;

            loop {
                if self.stop_signal.load(Ordering::Acquire) {
                    child.start_kill()?;
                    break;
                }

                if child.try_wait()?.is_some() {
                    break;
                }
            }

            if self.stop_signal.load(Ordering::Acquire) {
                child.start_kill()?;
                break;
            }

            info!("Waiting 5s before next launch.");
            tokio::time::sleep(Duration::from_secs(5)).await;
        }

        Ok(())
    }
}
