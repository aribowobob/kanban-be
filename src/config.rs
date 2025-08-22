use std::env;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub port: u16,
    pub jwt_secret: String,
    pub environment: String,
    pub frontend_urls: Vec<String>,
}

#[derive(Debug)]
pub enum ConfigError {
    MissingVariable(String),
    InvalidFormat(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::MissingVariable(var) => write!(f, "Missing environment variable: {}", var),
            ConfigError::InvalidFormat(msg) => write!(f, "Invalid format: {}", msg),
        }
    }
}

impl std::error::Error for ConfigError {}

impl AppConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        dotenv::dotenv().ok();
        
        let database_url = env::var("DATABASE_URL")
            .map_err(|_| ConfigError::MissingVariable("DATABASE_URL".to_string()))?;
        
        let jwt_secret = env::var("JWT_SECRET")
            .map_err(|_| ConfigError::MissingVariable("JWT_SECRET".to_string()))?;
        
        let environment = env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string());
        
        let port = env::var("SERVER_PORT")
            .unwrap_or_else(|_| "8080".to_string())
            .parse::<u16>()
            .map_err(|_| ConfigError::InvalidFormat("SERVER_PORT must be a valid port number".to_string()))?;
        
        // Parse allowed origins
        let frontend_urls = env::var("CORS_ALLOWED_ORIGINS")
            .unwrap_or_else(|_| "http://localhost:3001,https://kanban-fe.vercel.app".to_string())
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();
        
        Ok(AppConfig {
            database_url,
            jwt_secret,
            environment,
            port,
            frontend_urls,
        })
    }

    pub fn is_development(&self) -> bool {
        self.environment == "development"
    }
}
