use std::{
    io::{self, Read},
    path::PathBuf,
    process::Stdio,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use reqwest::Client;
use tokio::{fs::File, io::AsyncWriteExt, process, sync::Mutex};
use tracing::info;

use crate::{api::Version, environment::env_var_else};

pub struct LaunchData {
    pub use_aikar: bool,
    pub working_dir: PathBuf,
    pub version: Version,
    pub stop_signal: Arc<AtomicBool>,
    pub stopped_signal: Arc<AtomicBool>,
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
            "-Xms".to_owned() + &env_var_else("BLEEDINGEDGE_MIN_MEM", "1g"),
            "-Xmx".to_owned() + &env_var_else("BLEEDINGEDGE_MAX_MEM", "1g"),
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
            .arg("--nogui")
            .stdin(Stdio::piped());

        let mut interval = tokio::time::interval(Duration::from_secs(5));
        let mut poll_interval = tokio::time::interval(Duration::from_secs(1));
        let mut is_killing = false;
        let mut attempt_count = 0;

        loop {
            info!("Launching {:?}!", command);
            let mut child = command.spawn()?;
            let kill_bridge = Arc::new(AtomicBool::new(false));
            let stdin = Arc::new(Mutex::new(child.stdin.take().unwrap()));

            let kill_bridge_clone = kill_bridge.clone();
            let stdin_clone = stdin.clone();

            tokio::spawn(async move {
                let mut self_stdin = io::stdin();

                loop {
                    if kill_bridge_clone.load(Ordering::Acquire) {
                        break;
                    }

                    let mut data = vec![0u8; 1024];
                    let read = self_stdin.read(&mut data);

                    if let Ok(read_bytes) = read {
                        if read_bytes < 1 {
                            continue;
                        }

                        if let Err(_) = stdin_clone.lock().await.write(&data[..read_bytes]).await {
                            break;
                        }
                    } else {
                        break;
                    }
                }
            });

            loop {
                poll_interval.tick().await;

                if child.try_wait()?.is_some() {
                    kill_bridge.store(true, Ordering::Release);
                    self.stopped_signal.store(true, Ordering::Release);
                    break;
                }

                if self.stop_signal.load(Ordering::Acquire) && !is_killing {
                    is_killing = true;
                }

                if is_killing {
                    attempt_count += 1;

                    // Give the server a maximum of 3 minutes to save world data
                    if attempt_count > 180 {
                        let _ = child.kill().await;
                    } else {
                        let mut stdin_lock = stdin.lock().await;
                        stdin_lock.write("stop\n".as_bytes()).await?;
                        stdin_lock.flush().await?;
                    }

                    continue;
                }
            }

            if self.stop_signal.load(Ordering::Acquire) {
                break;
            }

            info!("Waiting 5s before next launch.");
            interval.tick().await;
        }

        Ok(())
    }
}
