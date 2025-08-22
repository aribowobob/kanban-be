use std::env;
use anyhow::{Result, Context};

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub port: u16,
    pub jwt_secret: String,
    pub environment: String,
    pub frontend_urls: Vec<String>,
    pub cloudinary_cloud_name: Option<String>,
    pub cloudinary_api_key: Option<String>,
    pub cloudinary_api_secret: Option<String>,
}

impl AppConfig {
    pub fn from_env() -> Result<Self> {
        dotenv::dotenv().ok();

        let database_url = env::var("DATABASE_URL")
            .context("DATABASE_URL must be set")?;

        let port = env::var("PORT")
            .unwrap_or_else(|_| "8080".to_string())
            .parse::<u16>()
            .context("PORT must be a valid number")?;

        let jwt_secret = env::var("JWT_SECRET")
            .context("JWT_SECRET must be set")?;

        let environment = env::var("ENVIRONMENT")
            .unwrap_or_else(|_| "development".to_string());

        let frontend_urls_str = env::var("FRONTEND_URLS")
            .unwrap_or_else(|_| "http://localhost:3000".to_string());
        
        let frontend_urls: Vec<String> = frontend_urls_str
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();

        // Cloudinary configuration (optional for now)
        let cloudinary_cloud_name = env::var("CLOUDINARY_CLOUD_NAME").ok();
        let cloudinary_api_key = env::var("CLOUDINARY_API_KEY").ok();
        let cloudinary_api_secret = env::var("CLOUDINARY_API_SECRET").ok();

        // Validation
        if jwt_secret.len() < 32 {
            return Err(anyhow::anyhow!("JWT_SECRET must be at least 32 characters long for security"));
        }

        if !database_url.starts_with("postgresql://") && !database_url.starts_with("postgres://") {
            return Err(anyhow::anyhow!("DATABASE_URL must be a valid PostgreSQL connection string"));
        }

        Ok(AppConfig {
            database_url,
            port,
            jwt_secret,
            environment,
            frontend_urls,
            cloudinary_cloud_name,
            cloudinary_api_key,
            cloudinary_api_secret,
        })
    }

    pub fn is_development(&self) -> bool {
        self.environment == "development"
    }

    pub fn is_production(&self) -> bool {
        self.environment == "production"
    }

    pub fn has_cloudinary_config(&self) -> bool {
        self.cloudinary_cloud_name.is_some() 
            && self.cloudinary_api_key.is_some() 
            && self.cloudinary_api_secret.is_some()
    }

    pub fn validate(&self) -> Result<()> {
        // Additional runtime validations
        if self.port < 1024 && !cfg!(test) {
            log::warn!("Port {} is below 1024, make sure you have proper permissions", self.port);
        }

        if self.frontend_urls.is_empty() {
            return Err(anyhow::anyhow!("At least one frontend URL must be specified"));
        }

        for url in &self.frontend_urls {
            if !url.starts_with("http://") && !url.starts_with("https://") {
                return Err(anyhow::anyhow!("Frontend URL '{}' must start with http:// or https://", url));
            }
        }

        log::info!("✅ Configuration validation passed");
        log::info!("📊 Environment: {}", self.environment);
        log::info!("🔌 Port: {}", self.port);
        log::info!("🌐 Frontend URLs: {:?}", self.frontend_urls);
        log::info!("☁️ Cloudinary configured: {}", self.has_cloudinary_config());

        Ok(())
    }
}
