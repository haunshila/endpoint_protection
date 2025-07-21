use tokio::sync::mpsc;
use log::{info};

pub mod config;
pub mod file_monitor;

pub async fn main_logic() -> Result<(), Box<dyn std::error::Error>> {
    let settings = config::Config::load_settings("config/settings.toml")?;

    let (tx, mut rx) = mpsc::channel(100);

    let _watcher = file_monitor::monitor_directories(&settings.paths_to_monitor, tx)?;

    // Spawn background task to receive events
    tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            info!("Event received: {:?}", event);
        }
    });

    // Keep the main task alive
    tokio::signal::ctrl_c().await?;
    log::info!("Shutting down due to Ctrl+C");

    Ok(())
}

