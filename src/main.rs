
fn main() {
    env_logger::init(); // Initialize logging from environment

    if let Err(e) = endpoint_protection_agent::main_logic() {
        log::error!("Application error: {}", e);
    }
}