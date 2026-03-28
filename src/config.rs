use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub default_profile: String,
    pub profiles_dir: PathBuf,
    pub sandbox_dir: PathBuf,
    pub log_dir: PathBuf,
    pub data_dir: PathBuf,
    pub proxy_enabled: bool,
    pub proxy_port: u16,
    pub max_commands_per_hour: u32,
    pub max_concurrent_agents: u32,
}

impl Default for Config {
    fn default() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
        Self {
            host: "127.0.0.1".to_string(),
            port: 7575,
            username: "bastion".to_string(),
            password: "bastion".to_string(),
            default_profile: "default".to_string(),
            profiles_dir: PathBuf::from("/usr/local/etc/bastion/profiles"),
            sandbox_dir: PathBuf::from("/tmp/bastion_sandbox"),
            log_dir: home.join(".bastion/logs"),
            data_dir: home.join(".bastion/data"),
            proxy_enabled: true,
            proxy_port: 8080,
            max_commands_per_hour: 100,
            max_concurrent_agents: 10,
        }
    }
}
