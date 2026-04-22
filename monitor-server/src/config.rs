use serde::Deserialize;

/// 应用配置，从环境变量加载（支持 .env 文件）。
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub database_url: String,
    pub redis_url: String,
    pub jwt_secret: String,
    #[serde(default)]
    pub ai_api_key: String,
    #[serde(default = "default_server_port")]
    pub server_port: u16,
    #[serde(default = "default_sse_port")]
    pub sse_port: u16,
}

fn default_server_port() -> u16 {
    8080
}

fn default_sse_port() -> u16 {
    8081
}

impl Config {
    pub fn from_env() -> Result<Self, config::ConfigError> {
        config::Config::builder()
            .add_source(config::Environment::default())
            .build()?
            .try_deserialize()
    }
}
