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
    /// 1. config/{profile}.toml (profile-specific configuration)
    /// 2. Environment variables with prefix APP_ (e.g., APP_WINDOW__WIDTH=1920)
    ///
    /// Config files are searched for in:
    /// 1. Next to the executable (target/debug/config or target/release/config)
    /// 2. In the current directory (./config)
    pub fn load(profile: &str) -> Result<Self, ConfigError> {
        // Find config directory - try relative to executable first, then current directory
        let config_dir = Self::find_config_dir();

        let mut builder = Config::builder();

        // Add profile-specific configuration
        if let Some(ref dir) = config_dir {
            let profile_path = dir.join(profile);
            builder = builder.add_source(File::from(profile_path.as_path()).required(false));
        } else {
            builder =
                builder.add_source(File::with_name(&format!("config/{}", profile)).required(false));
        }

        // Add environment variables with APP_ prefix
        // Use __ as separator for nested fields (e.g., APP_WINDOW__WIDTH)
        builder = builder.add_source(
            Environment::with_prefix("APP")
                .separator("__")
                .try_parsing(true),
        );

        // Set the profile
        let config = builder.set_override("profile", profile)?.build()?;

        config.try_deserialize()
    }

    /// Finds the config directory by searching in multiple locations
    fn find_config_dir() -> Option<std::path::PathBuf> {
        // Try to find config dir relative to executable
        if let Ok(exe_path) = std::env::current_exe()
            && let Some(exe_dir) = exe_path.parent()
        {
            let config_dir = exe_dir.join("config");
            if config_dir.exists() {
                return Some(config_dir);
            }
        }

        // Fall back to current directory
        let cwd_config = std::path::PathBuf::from("config");
        if cwd_config.exists() {
            return Some(cwd_config);
        }

        None
    }

    /// Loads configuration using the APP_PROFILE environment variable,
    /// defaulting to "release"
    pub fn load_from_env() -> Result<Self, ConfigError> {
        let profile = std::env::var("APP_PROFILE").unwrap_or_else(|_| "release".to_string());
        Self::load(&profile)
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self::load("release").unwrap_or_else(|_| Self {
            profile: "release".to_string(),
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
