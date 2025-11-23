//! Application configuration
//!
//! Supports multiple profiles (debug, release) with different settings.

use config::{Config, ConfigError, Environment, File};
use serde::{Deserialize, Serialize};

/// Window configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowConfig {
    /// Window title
    pub title: String,
    /// Window width
    pub width: f64,
    /// Window height
    pub height: f64,
    /// Whether the window should be fullscreen
    pub fullscreen: bool,
    /// Whether the window should be resizable
    pub resizable: bool,
    /// Whether the window should be decorated (has title bar, borders, etc.)
    pub decorated: bool,
    /// Whether to enable vsync
    pub vsync: bool,
}

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// The active profile (debug, release, etc.)
    pub profile: String,
    /// Window configuration
    pub window: WindowConfig,
}

impl AppConfig {
    /// Loads configuration based on the specified profile
    ///
    /// Profiles are loaded from config files in the following order:
    /// 1. config/default.toml (base configuration)
    /// 2. config/{profile}.toml (profile-specific overrides)
    /// 3. Environment variables with prefix APP_ (e.g., APP_WINDOW__WIDTH=1920)
    pub fn load(profile: &str) -> Result<Self, ConfigError> {
        let config = Config::builder()
            // Start with default configuration
            .add_source(File::with_name("config/default").required(false))
            // Add profile-specific configuration
            .add_source(File::with_name(&format!("config/{}", profile)).required(false))
            // Add environment variables with APP_ prefix
            // Use __ as separator for nested fields (e.g., APP_WINDOW__WIDTH)
            .add_source(
                Environment::with_prefix("APP")
                    .separator("__")
                    .try_parsing(true),
            )
            // Set the profile
            .set_override("profile", profile)?
            .build()?;

        config.try_deserialize()
    }

    /// Loads configuration using the APP_PROFILE environment variable,
    /// defaulting to "debug" if not set
    pub fn load_from_env() -> Result<Self, ConfigError> {
        let profile = std::env::var("APP_PROFILE").unwrap_or_else(|_| "debug".to_string());
        Self::load(&profile)
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self::load("debug").unwrap_or_else(|_| Self {
            profile: "debug".to_string(),
            window: WindowConfig {
                title: "Oil Pool Game".to_string(),
                width: 800.0,
                height: 600.0,
                fullscreen: false,
                resizable: true,
                decorated: true,
                vsync: true,
            },
        })
    }
}
