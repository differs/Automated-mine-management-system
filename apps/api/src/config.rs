#[derive(Clone, Debug, PartialEq)]
pub enum DispatchMode {
    PureAlgorithm,
    AiEnhanced,
}

impl DispatchMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            DispatchMode::PureAlgorithm => "pure_algorithm",
            DispatchMode::AiEnhanced => "ai_enhanced",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "ai_enhanced" => DispatchMode::AiEnhanced,
            _ => DispatchMode::PureAlgorithm,
        }
    }
}

#[derive(Clone, Debug)]
pub struct AiConfig {
    pub enabled: bool,
    pub api_url: String,
    pub api_key: String,
    pub model: String,
}

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub redis_url: String,
    pub rust_log: String,
    pub jwt_secret: String,
    pub dispatch_mode: DispatchMode,
    pub ai: AiConfig,
}

impl AppConfig {
    pub fn from_env() -> Self {
        let mode_str = std::env::var("DISPATCH_MODE")
            .unwrap_or_else(|_| "pure_algorithm".to_string());

        Self {
            host: std::env::var("APP_HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            port: std::env::var("APP_PORT")
                .ok()
                .and_then(|value| value.parse::<u16>().ok())
                .unwrap_or(3000),
            database_url: std::env::var("DATABASE_URL").unwrap_or_else(|_| {
                "postgres://postgres:postgres@localhost:5432/auto_mining_system".to_string()
            }),
            redis_url: std::env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://localhost:6379".to_string()),
            rust_log: std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "api=debug,tower_http=info".to_string()),
            jwt_secret: std::env::var("JWT_SECRET")
                .unwrap_or_else(|_| "replace-me-in-production".to_string()),
            dispatch_mode: DispatchMode::from_str(&mode_str),
            ai: AiConfig {
                enabled: std::env::var("AI_ENABLED").ok().as_deref() == Some("true"),
                api_url: std::env::var("AI_API_URL")
                    .unwrap_or_else(|_| "http://localhost:8000/v1/chat/completions".to_string()),
                api_key: std::env::var("AI_API_KEY")
                    .unwrap_or_else(|_| String::new()),
                model: std::env::var("AI_MODEL")
                    .unwrap_or_else(|_| "openPangu-2.0-Flash".to_string()),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dispatch_mode_as_str() {
        assert_eq!(DispatchMode::PureAlgorithm.as_str(), "pure_algorithm");
        assert_eq!(DispatchMode::AiEnhanced.as_str(), "ai_enhanced");
    }

    #[test]
    fn test_dispatch_mode_from_str() {
        assert_eq!(DispatchMode::from_str("pure_algorithm"), DispatchMode::PureAlgorithm);
        assert_eq!(DispatchMode::from_str("ai_enhanced"), DispatchMode::AiEnhanced);
        assert_eq!(DispatchMode::from_str("unknown"), DispatchMode::PureAlgorithm);
        assert_eq!(DispatchMode::from_str(""), DispatchMode::PureAlgorithm);
    }

    #[test]
    fn test_dispatch_mode_equality() {
        assert_eq!(DispatchMode::PureAlgorithm, DispatchMode::PureAlgorithm);
        assert_ne!(DispatchMode::PureAlgorithm, DispatchMode::AiEnhanced);
    }
}
