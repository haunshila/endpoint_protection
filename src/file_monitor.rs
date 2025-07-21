use notify::{RecommendedWatcher, RecursiveMode, Watcher, Event};
use std::path::PathBuf;
use std::sync::mpsc::{Sender};
use log::{info, error};

pub fn monitor_directories(
    paths: &[String],
    tx: Sender<Event>,
) -> notify::Result<RecommendedWatcher> {
    let mut watcher: RecommendedWatcher = notify::recommended_watcher({
        let tx = tx.clone();
        move |res| {
            match res {
                Ok(event) => {
                    if let Err(e) = tx.send(event) {
                        error!("Failed to send event: {}", e);
                    }
                }
                Err(e) => error!("Watcher error: {}", e),
            }
        }
    })?;

    for path_str in paths {
        let path = PathBuf::from(path_str);
        watcher.watch(&path, RecursiveMode::Recursive)?;
        info!("Started watching: {:?}", path);
    }

    Ok(watcher) // Return to keep it alive in calling scope
}