use serde::Deserialize;

/// 应用配置，从环境变量加载（支持 .env 文件）。
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub database_url: String,
    pub redis_url: String,
    pub jwt_secret: String,
    #[serde(default)]
    pub ai_api_key: String,
    #[serde(default = "default_ai_api_base")]
    pub ai_api_base: String,
    #[serde(default = "default_ai_model")]
    pub ai_model: String,
    #[serde(default = "default_ai_enabled")]
    pub ai_enabled: bool,
    #[serde(default = "default_sourcemap_dir")]
    pub sourcemap_dir: String,
    #[serde(default = "default_server_port")]
    pub server_port: u16,
    #[serde(default = "default_sse_port")]
    pub sse_port: u16,
    /// 允许的 CORS 来源（逗号分隔）。为空时允许所有来源（仅限开发环境）。
    #[serde(default)]
    pub cors_origins: String,
}

fn default_server_port() -> u16 {
    8080
}
fn default_sse_port() -> u16 {
    8081
}
fn default_ai_api_base() -> String {
    "https://api.deepseek.com/v1".into()
}
fn default_ai_model() -> String {
    "deepseek-chat".into()
}
fn default_ai_enabled() -> bool {
    true
}
fn default_sourcemap_dir() -> String {
    "./data/sourcemaps".into()
}

impl Config {
    pub fn from_env() -> Result<Self, config::ConfigError> {
        config::Config::builder()
            .add_source(config::Environment::default())
            .build()?
            .try_deserialize()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_server_port() {
        assert_eq!(default_server_port(), 8080);
    }

    #[test]
    fn test_default_sse_port() {
        assert_eq!(default_sse_port(), 8081);
    }

    #[test]
    fn test_default_ai_api_base() {
        assert_eq!(default_ai_api_base(), "https://api.deepseek.com/v1");
    }

    #[test]
    fn test_default_ai_model() {
        assert_eq!(default_ai_model(), "deepseek-chat");
    }

    #[test]
    fn test_default_ai_enabled() {
        assert!(default_ai_enabled());
    }

    #[test]
    fn test_default_sourcemap_dir() {
        assert_eq!(default_sourcemap_dir(), "./data/sourcemaps");
    }

    #[test]
    fn test_config_deserialize_with_all_fields() {
        let config: Config = serde_json::from_str(
            r#"{
            "database_url": "postgres://localhost/test",
            "redis_url": "redis://localhost",
            "jwt_secret": "secret123",
            "ai_api_key": "key123",
            "ai_api_base": "https://custom.api/v1",
            "ai_model": "gpt-4",
            "ai_enabled": false,
            "sourcemap_dir": "/tmp/maps",
            "server_port": 3000,
            "sse_port": 3001,
            "cors_origins": "http://localhost:3000"
        }"#,
        )
        .unwrap();
        assert_eq!(config.database_url, "postgres://localhost/test");
        assert_eq!(config.redis_url, "redis://localhost");
        assert_eq!(config.jwt_secret, "secret123");
        assert_eq!(config.ai_api_key, "key123");
        assert_eq!(config.ai_api_base, "https://custom.api/v1");
        assert_eq!(config.ai_model, "gpt-4");
        assert!(!config.ai_enabled);
        assert_eq!(config.sourcemap_dir, "/tmp/maps");
        assert_eq!(config.server_port, 3000);
        assert_eq!(config.sse_port, 3001);
        assert_eq!(config.cors_origins, "http://localhost:3000");
    }

    #[test]
    fn test_config_deserialize_with_defaults() {
        let config: Config = serde_json::from_str(
            r#"{
            "database_url": "postgres://localhost/test",
            "redis_url": "redis://localhost",
            "jwt_secret": "secret123"
        }"#,
        )
        .unwrap();
        assert_eq!(config.ai_api_key, "");
        assert_eq!(config.ai_api_base, "https://api.deepseek.com/v1");
        assert_eq!(config.ai_model, "deepseek-chat");
        assert!(config.ai_enabled);
        assert_eq!(config.sourcemap_dir, "./data/sourcemaps");
        assert_eq!(config.server_port, 8080);
        assert_eq!(config.sse_port, 8081);
        assert_eq!(config.cors_origins, "");
    }

    #[test]
    fn test_config_clone() {
        let config = Config {
            database_url: "pg://localhost".into(),
            redis_url: "redis://localhost".into(),
            jwt_secret: "secret".into(),
            ai_api_key: String::new(),
            ai_api_base: default_ai_api_base(),
            ai_model: default_ai_model(),
            ai_enabled: default_ai_enabled(),
            sourcemap_dir: default_sourcemap_dir(),
            server_port: default_server_port(),
            sse_port: default_sse_port(),
            cors_origins: String::new(),
        };
        let cloned = config.clone();
        assert_eq!(cloned.database_url, config.database_url);
        assert_eq!(cloned.server_port, config.server_port);
    }
}
