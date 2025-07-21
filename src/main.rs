
#[tokio::main]
async fn main() {
    env_logger::init(); // Or tracing_subscriber
    if let Err(e) = endpoint_protection_agent::main_logic().await {
        log::error!("Application error: {}", e);
    }
}