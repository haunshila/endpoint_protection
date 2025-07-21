use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use tokio::sync::mpsc::Sender;
use log::{info, error};

pub fn monitor_directories(
    paths: &[String],
    tx: Sender<Event>,
) -> notify::Result<RecommendedWatcher> {
    let mut watcher = notify::recommended_watcher({
        let tx = tx.clone();
        move |res| {
            if let Ok(event) = res {
                let _ = tx.blocking_send(event);
            } else if let Err(e) = res {
                error!("Watcher error: {}", e);
            }
        }
    })?;

    for path_str in paths {
        let path = PathBuf::from(path_str);
        watcher.watch(&path, RecursiveMode::Recursive)?;
        info!("Started watching: {:?}", path);
    }

    Ok(watcher)
}