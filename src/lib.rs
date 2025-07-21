use std::sync::mpsc::channel;
use std::thread;
use log::{info};

pub mod config;
pub mod file_monitor;

pub fn main_logic() -> Result<(), Box<dyn std::error::Error>> {
    let settings = config::Config::load_settings("config/settings.toml")?;
    let (tx, rx) = channel();

    let _watcher = file_monitor::monitor_directories(&settings.paths_to_monitor, tx)?;

    // Spawn thread to handle events
    let handle = thread::spawn(move || {
        for event in rx {
            info!("Event received: {:?}", event);
        }
    });

    handle.join().expect("Watcher thread panicked");

    Ok(())
}
