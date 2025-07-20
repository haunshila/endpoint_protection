use log::{info, error, warn};
use notify::{Event, EventKind, RecursiveMode};
use notify_debouncer_full::{new_debouncer, DebouncedEvent, Debouncer, FileIdMap};
use std::path::{PathBuf};
use std::time::Duration; // Removed Instant
use tokio::sync::mpsc; // For sending events to the main loop

/// Represents a file system event that occurred.
#[derive(Debug, PartialEq, Eq)] // Added PartialEq and Eq for easier comparison in tests
pub enum FileSystemEvent {
    Created(PathBuf),
    Deleted(PathBuf),
    Modified(PathBuf),
    // Removed Renamed variant as it's no longer explicitly constructed
    Other(Event), // For events not directly mapped
}

/// A struct to manage file system monitoring.
pub struct FileMonitor {
    paths_to_watch: Vec<PathBuf>,
    // Debouncer needs to be held to keep the watcher alive
    _debouncer: Debouncer<notify::RecommendedWatcher, FileIdMap>,
}

impl FileMonitor {
    /// Creates a new `FileMonitor` instance and its associated event receiver.
    ///
    /// `paths`: A vector of paths to directories or files to monitor.
    /// Returns a tuple containing the `FileMonitor` instance and an MPSC receiver for file system events.
    pub fn new(
        paths: Vec<PathBuf>,
    ) -> Result<(Self, mpsc::Receiver<FileSystemEvent>), Box<dyn std::error::Error>> {
        let (tx_internal, rx_internal) = mpsc::channel(100); // Internal channel for debouncer to send events to

        // Create a debouncer with a 2-second debounce time
        let debouncer: Debouncer<notify::RecommendedWatcher, FileIdMap> = new_debouncer(
            Duration::from_secs(2),
            None,
            move |result: Result<Vec<DebouncedEvent>, Vec<notify::Error>>| {
                match result {
                    Ok(events) => {
                        for event in events {
                            let current_paths = event.paths.clone();
                            let event_kind = &event.event.kind;

                            let fs_event = match event_kind {
                                EventKind::Create(_) => {
                                    current_paths.first().map(|p| FileSystemEvent::Created(p.clone()))
                                },
                                EventKind::Remove(_) => {
                                    current_paths.first().map(|p| FileSystemEvent::Deleted(p.clone()))
                                },
                                EventKind::Modify(_) => {
                                    current_paths.first().map(|p| FileSystemEvent::Modified(p.clone()))
                                },
                                EventKind::Access(_) => {
                                    current_paths.first().map(|p| {
                                        info!("File accessed: {}", p.display());
                                        FileSystemEvent::Other(event.event.clone())
                                    })
                                },
                                EventKind::Other => {
                                    Some(FileSystemEvent::Other(event.event.clone()))
                                },
                                _ => { // Catch-all for any remaining or future variants
                                    warn!("Unhandled or unrecognized file system event kind: {:?}", event_kind);
                                    Some(FileSystemEvent::Other(event.event.clone()))
                                },
                            };

                            if let Some(fs_event) = fs_event {
                                if let Err(e) = tx_internal.blocking_send(fs_event) {
                                    error!("Failed to send file system event to main loop: {}", e);
                                }
                            }
                        }
                    },
                    Err(e) => error!("File system watch error: {:?}", e),
                }
            },
        )?;

        Ok((
            FileMonitor {
                paths_to_watch: paths,
                _debouncer: debouncer,
            },
            rx_internal, // Return the receiver
        ))
    }

    /// Starts monitoring the configured paths.
    /// This function should be called after `FileMonitor::new` and the returned
    /// receiver is being listened to in a separate task.
    pub fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Starting file system monitoring for paths: {:?}", self.paths_to_watch);

        for path in &self.paths_to_watch {
            if path.is_dir() {
                info!("Watching directory: {}", path.display());
                self._debouncer.watch(path, RecursiveMode::Recursive)?;
            } else if path.is_file() {
                info!("Watching file: {}", path.display());
                self._debouncer.watch(path, RecursiveMode::NonRecursive)?;
            } else {
                warn!("Path does not exist or is not a file/directory: {}", path.display());
            }
        }
        info!("File system monitoring initialized.");
        Ok(())
    }
}