use log::{info, error};
use sysinfo::{System};
use tokio::time::{sleep, Duration};
use tokio::sync::mpsc; // For inter-task communication

mod file_monitor; // Declare the new file_monitor module
use file_monitor::{FileMonitor, FileSystemEvent};

// Conditionally compile the tests module only when running tests
#[cfg(test)]
mod tests; // Declare the tests module

// A simple configuration struct
#[derive(Debug, serde::Deserialize)]
struct AgentConfig {
    agent_id: String,
    check_interval_seconds: u64,
    #[serde(default = "default_server_url")]
    server_url: String,
    #[serde(default)] // Allow this field to be optional in config
    paths_to_monitor: Vec<String>,
}

fn default_server_url() -> String {
    "http://localhost:8080/api/v1/telemetry".to_string()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .init();
    info!("Starting Endpoint Protection Agent...");

    // 2. Load configuration
    let config = match config::Config::builder()
        .add_source(config::File::with_name("config/settings.toml"))
        .build()
    {
        Ok(cfg) => {
            match cfg.try_deserialize::<AgentConfig>() {
                Ok(agent_cfg) => {
                    info!("Configuration loaded successfully: {:?}", agent_cfg);
                    agent_cfg
                },
                Err(e) => {
                    error!("Failed to deserialize configuration: {}", e);
                    // Provide a default or exit
                    AgentConfig {
                        agent_id: "default-agent-001".to_string(),
                        check_interval_seconds: 60,
                        server_url: default_server_url(),
                        paths_to_monitor: vec![], // Default empty
                    }
                }
            }
        },
        Err(e) => {
            error!("Failed to load configuration file: {}. Using default settings.", e);
            // Provide a default or exit
            AgentConfig {
                agent_id: "default-agent-001".to_string(),
                check_interval_seconds: 60,
                server_url: default_server_url(),
                paths_to_monitor: vec![], // Default empty
            }
        }
    };

    // 3. Initialize system information library
    let mut sys = System::new_all();
    sys.refresh_all(); // Initial refresh

    info!("Agent ID: {}", config.agent_id);
    info!("Monitoring interval: {} seconds", config.check_interval_seconds);
    info!("Server URL: {}", config.server_url);

    // 4. Setup File System Monitoring
    let mut rx_fs_events: Option<mpsc::Receiver<FileSystemEvent>> = None;
    let paths_to_monitor_parsed: Vec<std::path::PathBuf> = config.paths_to_monitor.iter()
        .map(std::path::PathBuf::from)
        .collect();

    if !paths_to_monitor_parsed.is_empty() {
        match FileMonitor::new(paths_to_monitor_parsed) {
            Ok((mut file_monitor, receiver)) => {
                if let Err(e) = file_monitor.start() {
                    error!("Failed to start file system monitor: {}", e);
                } else {
                    info!("File system monitoring module initialized and started.");
                    rx_fs_events = Some(receiver); // Store the receiver
                }
            },
            Err(e) => error!("Failed to create file system monitor: {}", e),
        }
    } else {
        info!("No paths configured for file system monitoring.");
    }


    // Main agent loop
    loop {
        // Use tokio::select! to handle both periodic checks and file system events
        tokio::select! {
            // Regular system check interval
            _ = sleep(Duration::from_secs(config.check_interval_seconds)) => {
                info!("Performing system check...");

                // Refresh system information
                sys.refresh_all();

                // Example: Log CPU and Memory usage
                info!("CPU Usage: {:.2}%", sys.global_cpu_usage());
                info!("Total Memory: {} MB", sys.total_memory() / 1024 / 1024);
                info!("Used Memory: {} MB", sys.used_memory() / 1024 / 1024);

                // --- Placeholder for other monitoring and detection logic ---
                // - Process monitoring (iterating `sys.processes()`)
                // - Network connection monitoring
                // - Threat detection algorithms
                // - Communication with a central server (sending telemetry, receiving commands)

                // Simulate sending telemetry data (replace with actual HTTP/GRPC call)
                info!("Simulating sending telemetry data to {}", config.server_url);
                // let client = reqwest::Client::new();
                // let res = client.post(&config.server_url)
                //     .json(&serde_json::json!({
                //         "agent_id": config.agent_id,
                //         "timestamp": chrono::Utc::now().to_rfc3339(),
                //         "cpu_usage": sys.global_cpu_info().cpu_usage(),
                //         "used_memory_mb": sys.used_memory() / 1024 / 1024,
                //         // ... more telemetry data
                //     }))
                //     .send()
                //     .await;

                // match res {
                //     Ok(response) => info!("Telemetry sent, server responded with: {:?}", response.status()),
                //     Err(e) => error!("Failed to send telemetry: {}", e),
                // }
            }
            // File system events (only if the receiver exists)
            // The `if let Some(ref mut rx)` pattern allows us to conditionally await on the receiver.
            Some(event) = async {
                if let Some(ref mut rx) = rx_fs_events {
                    rx.recv().await
                } else {
                    // If no receiver, yield to prevent busy-waiting, or use `pending()`
                    // to effectively remove this branch from selection.
                    // For `tokio::select!`, `futures::future::pending().await` is a good way
                    // to make a branch never complete if the Option is None.
                    // However, for simplicity here, we'll just return None.
                    // In a real application, you might use a more sophisticated approach
                    // to ensure `tokio::select!` doesn't busy-wait if one branch is always None.
                    None
                }
            }, if rx_fs_events.is_some() => { // This `if` guard ensures the branch is only considered if `rx_fs_events` is `Some`
                info!("Received file system event: {:?}", event);
                // --- Placeholder for processing file system events ---
                // Here, you would analyze the event for suspicious activity.
                // For example:
                // match event {
                //     FileSystemEvent::Created(path) => {
                //         info!("New file created: {}", path.display());
                //         // Check file hash, run through a detection engine
                //     },
                //     FileSystemEvent::Modified(path) => {
                //         info!("File modified: {}", path.display());
                //         // Check for suspicious modifications
                //     },
                //     FileSystemEvent::Deleted(path) => {
                //         info!("File deleted: {}", path.display());
                //     },
                //     FileSystemEvent::Other(event_details) => { // Renamed removed
                //         info!("Other FS event: {:?}", event_details);
                //     }
                // }
            }
        }
    }
}