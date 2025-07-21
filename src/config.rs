use serde::Deserialize;
use std::fs;
use std::path::Path;
use std::error::Error;
#[derive(Debug, Deserialize)]
pub struct Config {
    pub agent_id: String,
    pub check_interval_seconds: u64,
    pub server_url: Option<String>,
    pub paths_to_monitor: Vec<String>,
}

impl Config {
    pub fn load_settings<P: AsRef<Path>>(path: P) -> Result<Config, Box<dyn Error>> {
        let content = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }
}
