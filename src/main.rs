// src/main.rs

use log::{info, error};
use sysinfo::{System};
use tokio::time::{sleep, Duration};

// A simple configuration struct
#[derive(Debug, serde::Deserialize)]
struct AgentConfig {
    agent_id: String,
    check_interval_seconds: u64,
    #[serde(default = "default_server_url")]
    server_url: String,
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
            }
        }
    };

    // 3. Initialize system information library
    let mut sys = System::new_all();
    sys.refresh_all(); // Initial refresh

    info!("Agent ID: {}", config.agent_id);
    info!("Monitoring interval: {} seconds", config.check_interval_seconds);
    info!("Server URL: {}", config.server_url);

    // Main agent loop
    loop {
        info!("Performing system check...");

        // Refresh system information
        sys.refresh_all();

        // Example: Log CPU and Memory usage
        info!("CPU Usage: {:.2}%", sys.global_cpu_usage());
        info!("Total Memory: {} MB", sys.total_memory() / 1024 / 1024);
        info!("Used Memory: {} MB", sys.used_memory() / 1024 / 1024);

        // --- Placeholder for actual monitoring and detection logic ---
        // In a real agent, you would have modules here for:
        // - File system monitoring (e.g., using `notify` crate or OS-specific APIs)
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


        // Wait for the next interval
        sleep(Duration::from_secs(config.check_interval_seconds)).await;
    }
}