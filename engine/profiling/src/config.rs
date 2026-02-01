//! Configuration system for the profiler.
//!
//! Provides multi-source configuration with the following priority:
//! 1. Environment variables (highest priority)
//! 2. Configuration file (YAML)
//! 3. Default values (lowest priority)
//!
//! # Examples
//!
//! Loading from a configuration file:
//!
//! ```rust,ignore
//! use agent_game_engine_profiling::ProfilerConfig;
//! use std::path::Path;
//!
//! let config = ProfilerConfig::from_file(Path::new("engine.config.yaml"))
//!     .unwrap_or_else(|_| ProfilerConfig::default());
//! ```
//!
//! Loading from environment variables:
//!
//! ```rust
//! use agent_game_engine_profiling::ProfilerConfig;
//!
//! let config = ProfilerConfig::from_env();
//! ```
//!
//! Merging environment overrides into a file-loaded config:
//!
//! ```rust,ignore
//! use agent_game_engine_profiling::ProfilerConfig;
//! use std::path::Path;
//!
//! let mut config = ProfilerConfig::from_file(Path::new("config.yaml"))?;
//! config.merge_env(); // Apply environment variable overrides
//! ```

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

#[cfg(feature = "config")]
use serde::{Deserialize, Serialize};

/// Error type for configuration loading and parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    /// Configuration file not found
    FileNotFound(String),
    /// Failed to parse configuration file
    ParseError(String),
    /// Invalid duration format
    InvalidDuration(String),
    /// I/O error
    IoError(String),
    /// Missing required field
    MissingField(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::FileNotFound(path) => write!(f, "Configuration file not found: {path}"),
            ConfigError::ParseError(msg) => write!(f, "Failed to parse configuration: {msg}"),
            ConfigError::InvalidDuration(s) => write!(f, "Invalid duration format: {s}"),
            ConfigError::IoError(msg) => write!(f, "I/O error: {msg}"),
            ConfigError::MissingField(field) => write!(f, "Missing required field: {field}"),
        }
    }
}

impl std::error::Error for ConfigError {}

/// Output format for profiling data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "config", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "config", serde(rename_all = "snake_case"))]
#[derive(Default)]
pub enum ProfileFormat {
    /// Chrome Tracing JSON format (for <chrome://tracing>)
    #[default]
    ChromeTrace,
    /// Generic JSON format
    Json,
}

/// Configuration for profiling data retention.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "config", derive(Serialize, Deserialize))]
pub struct RetentionConfig {
    /// Number of frames to keep in circular buffer
    pub circular_buffer_frames: usize,

    /// Whether to save data when a performance budget is exceeded
    pub save_on_budget_exceeded: bool,

    /// Whether to save data on crash (requires crash handler integration)
    pub save_on_crash: bool,
}

impl Default for RetentionConfig {
    fn default() -> Self {
        Self { circular_buffer_frames: 1000, save_on_budget_exceeded: true, save_on_crash: true }
    }
}

/// Configuration for the profiler.
///
/// Controls profiling behavior, persistence, output format, and performance budgets.
///
/// # Configuration Sources
///
/// Configuration can be loaded from multiple sources with the following priority:
/// 1. Environment variables (highest)
/// 2. YAML configuration file
/// 3. Default values (lowest)
///
/// # Environment Variables
///
/// - `PROFILE_ENABLE`: "true" or "false"
/// - `PROFILE_PERSIST`: "true" or "false"
/// - `PROFILE_DIR`: Output directory path
/// - `PROFILE_MAX_SIZE_MB`: Maximum file size in MB
/// - `PROFILE_FORMAT`: "`chrome_trace`" or "json"
/// - `PROFILE_BUFFER_FRAMES`: Circular buffer size
/// - `PROFILE_BUDGET_<name>`: Budget for specific scope (e.g., `PROFILE_BUDGET_GAME_LOOP=16.0ms`)
///
/// # YAML Format
///
/// ```yaml
/// profiling:
///   enabled: true
///   persist: true
///   output_dir: "profiling_data/"
///   max_file_size_mb: 100
///   format: chrome_trace
///   retention:
///     circular_buffer_frames: 1000
///     save_on_budget_exceeded: true
///     save_on_crash: true
///
/// budgets:
///   game_loop: 16.0ms
///   physics_step: 5.0ms
///   rendering: 8.0ms
/// ```
#[derive(Debug, Clone)]
#[cfg_attr(feature = "config", derive(Serialize, Deserialize))]
pub struct ProfilerConfig {
    /// Whether profiling is enabled
    pub enabled: bool,

    /// Whether to persist profiling data to disk
    pub persist_to_disk: bool,

    /// Output directory for profiling data
    pub output_dir: PathBuf,

    /// Maximum file size in MB before rotation
    pub max_file_size_mb: usize,

    /// Output format for profiling data
    pub format: ProfileFormat,

    /// Retention configuration
    pub retention: RetentionConfig,

    /// Performance budgets for specific scopes (scope name -> duration)
    #[cfg_attr(feature = "config", serde(with = "budget_serde"))]
    pub budgets: HashMap<String, Duration>,
}

impl Default for ProfilerConfig {
    fn default() -> Self {
        Self::default_dev()
    }
}

impl ProfilerConfig {
    /// Default configuration for development builds.
    ///
    /// Profiling enabled with reasonable defaults for development workflow.
    #[must_use]
    pub fn default_dev() -> Self {
        let mut budgets = HashMap::new();
        budgets.insert("game_loop".to_string(), Duration::from_millis(16)); // 60 FPS
        budgets.insert("physics_step".to_string(), Duration::from_millis(5));
        budgets.insert("rendering".to_string(), Duration::from_millis(8));
        budgets.insert("networking".to_string(), Duration::from_millis(2));

        Self {
            enabled: true,
            persist_to_disk: true,
            output_dir: PathBuf::from("profiling_data"),
            max_file_size_mb: 100,
            format: ProfileFormat::ChromeTrace,
            retention: RetentionConfig::default(),
            budgets,
        }
    }

    /// Default configuration for release builds.
    ///
    /// All profiling disabled to ensure zero overhead.
    #[must_use]
    pub fn default_release() -> Self {
        Self {
            enabled: false,
            persist_to_disk: false,
            output_dir: PathBuf::new(),
            max_file_size_mb: 0,
            format: ProfileFormat::ChromeTrace,
            retention: RetentionConfig {
                circular_buffer_frames: 0,
                save_on_budget_exceeded: false,
                save_on_crash: false,
            },
            budgets: HashMap::new(),
        }
    }

    /// Load configuration from a YAML file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the YAML configuration file
    ///
    /// # Returns
    ///
    /// Returns `Ok(ProfilerConfig)` if successful, or `Err(ConfigError)` if the file
    /// cannot be read or parsed.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use agent_game_engine_profiling::ProfilerConfig;
    /// use std::path::Path;
    ///
    /// let config = ProfilerConfig::from_file(Path::new("engine.config.yaml"))?;
    /// ```
    #[cfg(feature = "config")]
    pub fn from_file(path: &std::path::Path) -> Result<Self, ConfigError> {
        use std::fs;

        let contents = fs::read_to_string(path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                ConfigError::FileNotFound(path.display().to_string())
            } else {
                ConfigError::IoError(e.to_string())
            }
        })?;

        let root: ConfigFile =
            serde_yaml::from_str(&contents).map_err(|e| ConfigError::ParseError(e.to_string()))?;

        Ok(root.into())
    }

    /// Load configuration from environment variables.
    ///
    /// This method reads environment variables and constructs a configuration.
    /// If environment variables are not set, defaults from `default_dev()` are used.
    ///
    /// # Environment Variables
    ///
    /// - `PROFILE_ENABLE`: "true" or "false"
    /// - `PROFILE_PERSIST`: "true" or "false"
    /// - `PROFILE_DIR`: Output directory path
    /// - `PROFILE_MAX_SIZE_MB`: Maximum file size in MB
    /// - `PROFILE_FORMAT`: "`chrome_trace`" or "json"
    /// - `PROFILE_BUFFER_FRAMES`: Circular buffer size
    /// - `PROFILE_BUDGET_<name>`: Budget for specific scope
    ///
    /// # Examples
    ///
    /// ```rust
    /// use agent_game_engine_profiling::ProfilerConfig;
    ///
    /// let config = ProfilerConfig::from_env();
    /// ```
    #[must_use]
    pub fn from_env() -> Self {
        let mut config = Self::default_dev();

        if let Ok(val) = std::env::var("PROFILE_ENABLE") {
            config.enabled = parse_bool(&val);
        }

        if let Ok(val) = std::env::var("PROFILE_PERSIST") {
            config.persist_to_disk = parse_bool(&val);
        }

        if let Ok(val) = std::env::var("PROFILE_DIR") {
            config.output_dir = PathBuf::from(val);
        }

        if let Ok(val) = std::env::var("PROFILE_MAX_SIZE_MB") {
            if let Ok(size) = val.parse() {
                config.max_file_size_mb = size;
            }
        }

        if let Ok(val) = std::env::var("PROFILE_FORMAT") {
            match val.to_lowercase().as_str() {
                "chrome_trace" => config.format = ProfileFormat::ChromeTrace,
                "json" => config.format = ProfileFormat::Json,
                _ => {}
            }
        }

        if let Ok(val) = std::env::var("PROFILE_BUFFER_FRAMES") {
            if let Ok(frames) = val.parse() {
                config.retention.circular_buffer_frames = frames;
            }
        }

        // Load budget environment variables
        for (key, value) in std::env::vars() {
            if let Some(budget_name) = key.strip_prefix("PROFILE_BUDGET_") {
                if let Ok(duration) = parse_duration(&value) {
                    let budget_name_lower = budget_name.to_lowercase();
                    config.budgets.insert(budget_name_lower, duration);
                }
            }
        }

        config
    }

    /// Merge environment variable overrides into this configuration.
    ///
    /// This allows loading a base configuration from a file and then applying
    /// environment-specific overrides.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use agent_game_engine_profiling::ProfilerConfig;
    /// use std::path::Path;
    ///
    /// let mut config = ProfilerConfig::from_file(Path::new("config.yaml"))?;
    /// config.merge_env(); // Apply environment variable overrides
    /// ```
    pub fn merge_env(&mut self) {
        if let Ok(val) = std::env::var("PROFILE_ENABLE") {
            self.enabled = parse_bool(&val);
        }

        if let Ok(val) = std::env::var("PROFILE_PERSIST") {
            self.persist_to_disk = parse_bool(&val);
        }

        if let Ok(val) = std::env::var("PROFILE_DIR") {
            self.output_dir = PathBuf::from(val);
        }

        if let Ok(val) = std::env::var("PROFILE_MAX_SIZE_MB") {
            if let Ok(size) = val.parse() {
                self.max_file_size_mb = size;
            }
        }

        if let Ok(val) = std::env::var("PROFILE_FORMAT") {
            match val.to_lowercase().as_str() {
                "chrome_trace" => self.format = ProfileFormat::ChromeTrace,
                "json" => self.format = ProfileFormat::Json,
                _ => {}
            }
        }

        if let Ok(val) = std::env::var("PROFILE_BUFFER_FRAMES") {
            if let Ok(frames) = val.parse() {
                self.retention.circular_buffer_frames = frames;
            }
        }

        // Merge budget environment variables
        for (key, value) in std::env::vars() {
            if let Some(budget_name) = key.strip_prefix("PROFILE_BUDGET_") {
                if let Ok(duration) = parse_duration(&value) {
                    let budget_name_lower = budget_name.to_lowercase();
                    self.budgets.insert(budget_name_lower, duration);
                }
            }
        }
    }
}

// Helper types for YAML deserialization
#[cfg(feature = "config")]
#[derive(Debug, Deserialize)]
struct ConfigFile {
    profiling: ProfilingSection,
    #[serde(default)]
    budgets: HashMap<String, String>,
}

#[cfg(feature = "config")]
#[derive(Debug, Deserialize)]
struct ProfilingSection {
    #[serde(default = "default_true")]
    enabled: bool,
    #[serde(default = "default_true")]
    persist: bool,
    #[serde(default = "default_output_dir")]
    output_dir: String,
    #[serde(default = "default_max_size")]
    max_file_size_mb: usize,
    #[serde(default)]
    format: ProfileFormat,
    #[serde(default)]
    retention: RetentionConfig,
}

#[cfg(feature = "config")]
fn default_true() -> bool {
    true
}

#[cfg(feature = "config")]
fn default_output_dir() -> String {
    "profiling_data".to_string()
}

#[cfg(feature = "config")]
fn default_max_size() -> usize {
    100
}

#[cfg(feature = "config")]
impl From<ConfigFile> for ProfilerConfig {
    fn from(file: ConfigFile) -> Self {
        let mut budgets = HashMap::new();
        for (name, duration_str) in file.budgets {
            if let Ok(duration) = parse_duration(&duration_str) {
                budgets.insert(name, duration);
            }
        }

        Self {
            enabled: file.profiling.enabled,
            persist_to_disk: file.profiling.persist,
            output_dir: PathBuf::from(file.profiling.output_dir),
            max_file_size_mb: file.profiling.max_file_size_mb,
            format: file.profiling.format,
            retention: file.profiling.retention,
            budgets,
        }
    }
}

// Serde serialization for budgets (Duration -> String)
#[cfg(feature = "config")]
mod budget_serde {
    use super::*;
    use serde::{Deserializer, Serializer};

    pub fn serialize<S>(
        budgets: &HashMap<String, Duration>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(budgets.len()))?;
        for (k, v) in budgets {
            let duration_str = format_duration(v);
            map.serialize_entry(k, &duration_str)?;
        }
        map.end()
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<HashMap<String, Duration>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let map: HashMap<String, String> = HashMap::deserialize(deserializer)?;
        let mut result = HashMap::new();
        for (k, v) in map {
            match parse_duration(&v) {
                Ok(duration) => {
                    result.insert(k, duration);
                }
                Err(_) => {
                    return Err(serde::de::Error::custom(format!(
                        "Invalid duration format: {}",
                        v
                    )));
                }
            }
        }
        Ok(result)
    }
}

/// Parse a duration string.
///
/// Supports formats:
/// - "16ms" or "16.0ms" (milliseconds)
/// - "1s" or "1.0s" (seconds)
/// - "500us" (microseconds)
/// - "1000" (interpreted as milliseconds if no unit)
///
/// # Examples
///
/// ```rust
/// use agent_game_engine_profiling::parse_duration;
/// use std::time::Duration;
///
/// assert_eq!(parse_duration("16ms").unwrap(), Duration::from_millis(16));
/// assert_eq!(parse_duration("16.5ms").unwrap(), Duration::from_micros(16500));
/// assert_eq!(parse_duration("1s").unwrap(), Duration::from_secs(1));
/// assert_eq!(parse_duration("500us").unwrap(), Duration::from_micros(500));
/// ```
pub fn parse_duration(s: &str) -> Result<Duration, ConfigError> {
    let s = s.trim();

    // Try to parse with unit suffix
    if let Some(num_str) = s.strip_suffix("ms") {
        let num: f64 = num_str
            .trim()
            .parse()
            .map_err(|_| ConfigError::InvalidDuration(s.to_string()))?;
        let micros = (num * 1000.0) as u64;
        return Ok(Duration::from_micros(micros));
    }

    if let Some(num_str) = s.strip_suffix("us") {
        let num: f64 = num_str
            .trim()
            .parse()
            .map_err(|_| ConfigError::InvalidDuration(s.to_string()))?;
        return Ok(Duration::from_micros(num as u64));
    }

    if let Some(num_str) = s.strip_suffix('s') {
        let num: f64 = num_str
            .trim()
            .parse()
            .map_err(|_| ConfigError::InvalidDuration(s.to_string()))?;
        let micros = (num * 1_000_000.0) as u64;
        return Ok(Duration::from_micros(micros));
    }

    // No suffix - try to parse as float (interpret as milliseconds)
    if let Ok(num) = s.parse::<f64>() {
        let micros = (num * 1000.0) as u64;
        return Ok(Duration::from_micros(micros));
    }

    Err(ConfigError::InvalidDuration(s.to_string()))
}

/// Format a duration as a human-readable string.
///
/// # Examples
///
/// ```rust
/// use agent_game_engine_profiling::format_duration;
/// use std::time::Duration;
///
/// assert_eq!(format_duration(&Duration::from_millis(16)), "16.0ms");
/// assert_eq!(format_duration(&Duration::from_secs(1)), "1000.0ms");
/// ```
#[must_use]
pub fn format_duration(duration: &Duration) -> String {
    let micros = duration.as_micros();
    if micros >= 1000 {
        format!("{:.1}ms", micros as f64 / 1000.0)
    } else {
        format!("{micros}us")
    }
}

/// Parse a boolean from a string.
///
/// Accepts: "true", "1", "yes", "on" (case-insensitive) as true.
/// Everything else is false.
fn parse_bool(s: &str) -> bool {
    matches!(s.to_lowercase().as_str(), "true" | "1" | "yes" | "on")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Mutex to prevent env var tests from running concurrently
    static ENV_TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn test_parse_duration_ms() {
        assert_eq!(parse_duration("16ms").unwrap(), Duration::from_millis(16));
        assert_eq!(parse_duration("16.5ms").unwrap(), Duration::from_micros(16500));
        assert_eq!(parse_duration("0.5ms").unwrap(), Duration::from_micros(500));
    }

    #[test]
    fn test_parse_duration_us() {
        assert_eq!(parse_duration("500us").unwrap(), Duration::from_micros(500));
        assert_eq!(parse_duration("1000us").unwrap(), Duration::from_micros(1000));
    }

    #[test]
    fn test_parse_duration_s() {
        assert_eq!(parse_duration("1s").unwrap(), Duration::from_secs(1));
        assert_eq!(parse_duration("1.5s").unwrap(), Duration::from_micros(1_500_000));
    }

    #[test]
    fn test_parse_duration_no_unit() {
        assert_eq!(parse_duration("16").unwrap(), Duration::from_millis(16));
        assert_eq!(parse_duration("16.5").unwrap(), Duration::from_micros(16500));
    }

    #[test]
    fn test_parse_duration_invalid() {
        assert!(parse_duration("invalid").is_err());
        assert!(parse_duration("").is_err());
        assert!(parse_duration("16.5.5ms").is_err());
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(&Duration::from_millis(16)), "16.0ms");
        assert_eq!(format_duration(&Duration::from_micros(500)), "500us");
        assert_eq!(format_duration(&Duration::from_secs(1)), "1000.0ms");
    }

    #[test]
    fn test_parse_bool() {
        assert!(parse_bool("true"));
        assert!(parse_bool("TRUE"));
        assert!(parse_bool("1"));
        assert!(parse_bool("yes"));
        assert!(parse_bool("YES"));
        assert!(parse_bool("on"));
        assert!(!parse_bool("false"));
        assert!(!parse_bool("0"));
        assert!(!parse_bool("no"));
        assert!(!parse_bool("off"));
        assert!(!parse_bool("invalid"));
    }

    #[test]
    fn test_profiler_config_default() {
        let config = ProfilerConfig::default();
        assert!(config.enabled);
        assert!(config.persist_to_disk);
        assert_eq!(config.output_dir, PathBuf::from("profiling_data"));
        assert_eq!(config.max_file_size_mb, 100);
        assert_eq!(config.format, ProfileFormat::ChromeTrace);
        assert_eq!(config.retention.circular_buffer_frames, 1000);
        assert!(config.budgets.contains_key("game_loop"));
    }

    #[test]
    fn test_profiler_config_release() {
        let config = ProfilerConfig::default_release();
        assert!(!config.enabled);
        assert!(!config.persist_to_disk);
        assert_eq!(config.retention.circular_buffer_frames, 0);
        assert!(config.budgets.is_empty());
    }

    #[test]
    fn test_from_env_defaults() {
        let _lock = ENV_TEST_LOCK.lock();

        // Clear any existing profile env vars to ensure clean state
        std::env::remove_var("PROFILE_ENABLE");
        std::env::remove_var("PROFILE_PERSIST");
        std::env::remove_var("PROFILE_DIR");
        std::env::remove_var("PROFILE_MAX_SIZE_MB");
        std::env::remove_var("PROFILE_FORMAT");
        std::env::remove_var("PROFILE_BUFFER_FRAMES");

        let config = ProfilerConfig::from_env();
        // Should fall back to dev defaults when no env vars set
        assert!(config.enabled);
    }

    #[test]
    fn test_from_env_with_vars() {
        let _lock = ENV_TEST_LOCK.lock();

        // Save current env state and clear all profile env vars
        let old_enable = std::env::var("PROFILE_ENABLE").ok();
        let old_persist = std::env::var("PROFILE_PERSIST").ok();
        let old_dir = std::env::var("PROFILE_DIR").ok();
        let old_max_size = std::env::var("PROFILE_MAX_SIZE_MB").ok();
        let old_format = std::env::var("PROFILE_FORMAT").ok();
        let old_buffer = std::env::var("PROFILE_BUFFER_FRAMES").ok();

        // Clear any existing values first to ensure clean state
        std::env::remove_var("PROFILE_ENABLE");
        std::env::remove_var("PROFILE_PERSIST");
        std::env::remove_var("PROFILE_DIR");
        std::env::remove_var("PROFILE_MAX_SIZE_MB");
        std::env::remove_var("PROFILE_FORMAT");
        std::env::remove_var("PROFILE_BUFFER_FRAMES");

        // Now set test values
        std::env::set_var("PROFILE_ENABLE", "false");
        std::env::set_var("PROFILE_PERSIST", "false");
        std::env::set_var("PROFILE_DIR", "custom_dir");
        std::env::set_var("PROFILE_MAX_SIZE_MB", "200");
        std::env::set_var("PROFILE_FORMAT", "json");
        std::env::set_var("PROFILE_BUFFER_FRAMES", "500");
        std::env::set_var("PROFILE_BUDGET_CUSTOM_SCOPE", "10ms");

        let config = ProfilerConfig::from_env();

        assert!(!config.enabled, "config.enabled should be false but was {}", config.enabled);
        assert!(!config.persist_to_disk);
        assert_eq!(config.output_dir, PathBuf::from("custom_dir"));
        assert_eq!(config.max_file_size_mb, 200);
        assert_eq!(config.format, ProfileFormat::Json);
        assert_eq!(config.retention.circular_buffer_frames, 500);
        assert_eq!(config.budgets.get("custom_scope"), Some(&Duration::from_millis(10)));

        // Restore old state
        restore_or_remove("PROFILE_ENABLE", old_enable);
        restore_or_remove("PROFILE_PERSIST", old_persist);
        restore_or_remove("PROFILE_DIR", old_dir);
        restore_or_remove("PROFILE_MAX_SIZE_MB", old_max_size);
        restore_or_remove("PROFILE_FORMAT", old_format);
        restore_or_remove("PROFILE_BUFFER_FRAMES", old_buffer);
        std::env::remove_var("PROFILE_BUDGET_CUSTOM_SCOPE");
    }

    fn restore_or_remove(key: &str, old_value: Option<String>) {
        match old_value {
            Some(val) => std::env::set_var(key, val),
            None => std::env::remove_var(key),
        }
    }

    #[test]
    fn test_merge_env() {
        let _lock = ENV_TEST_LOCK.lock();

        // Save old state
        let old_enable = std::env::var("PROFILE_ENABLE").ok();
        let old_dir = std::env::var("PROFILE_DIR").ok();

        let mut config = ProfilerConfig::default_dev();
        config.enabled = true;
        config.output_dir = PathBuf::from("original_dir");

        std::env::set_var("PROFILE_ENABLE", "false");
        std::env::set_var("PROFILE_DIR", "overridden_dir");

        config.merge_env();

        assert!(!config.enabled);
        assert_eq!(config.output_dir, PathBuf::from("overridden_dir"));

        // Restore old state
        restore_or_remove("PROFILE_ENABLE", old_enable);
        restore_or_remove("PROFILE_DIR", old_dir);
    }

    #[cfg(feature = "config")]
    #[test]
    fn test_from_file_yaml() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        let yaml_content = r#"
profiling:
  enabled: true
  persist: true
  output_dir: "test_profiling_data"
  max_file_size_mb: 150
  format: chrome_trace
  retention:
    circular_buffer_frames: 2000
    save_on_budget_exceeded: true
    save_on_crash: false

budgets:
  game_loop: 16.0ms
  physics_step: 5.0ms
  rendering: 8.0ms
  networking: 2.0ms
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let config = ProfilerConfig::from_file(temp_file.path()).unwrap();

        assert!(config.enabled);
        assert!(config.persist_to_disk);
        assert_eq!(config.output_dir, PathBuf::from("test_profiling_data"));
        assert_eq!(config.max_file_size_mb, 150);
        assert_eq!(config.format, ProfileFormat::ChromeTrace);
        assert_eq!(config.retention.circular_buffer_frames, 2000);
        assert!(config.retention.save_on_budget_exceeded);
        assert!(!config.retention.save_on_crash);
        assert_eq!(config.budgets.get("game_loop"), Some(&Duration::from_millis(16)));
        assert_eq!(config.budgets.get("physics_step"), Some(&Duration::from_millis(5)));
    }

    #[cfg(feature = "config")]
    #[test]
    fn test_from_file_not_found() {
        let result = ProfilerConfig::from_file(std::path::Path::new("nonexistent.yaml"));
        assert!(matches!(result, Err(ConfigError::FileNotFound(_))));
    }

    #[cfg(feature = "config")]
    #[test]
    fn test_from_file_invalid_yaml() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        let invalid_yaml = "invalid: yaml: content: [[[";
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(invalid_yaml.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let result = ProfilerConfig::from_file(temp_file.path());
        assert!(matches!(result, Err(ConfigError::ParseError(_))));
    }

    #[test]
    fn test_config_error_display() {
        let err = ConfigError::FileNotFound("test.yaml".to_string());
        assert_eq!(err.to_string(), "Configuration file not found: test.yaml");

        let err = ConfigError::ParseError("syntax error".to_string());
        assert_eq!(err.to_string(), "Failed to parse configuration: syntax error");

        let err = ConfigError::InvalidDuration("16.0.0ms".to_string());
        assert_eq!(err.to_string(), "Invalid duration format: 16.0.0ms");
    }

    #[test]
    fn test_retention_config_default() {
        let retention = RetentionConfig::default();
        assert_eq!(retention.circular_buffer_frames, 1000);
        assert!(retention.save_on_budget_exceeded);
        assert!(retention.save_on_crash);
    }

    #[test]
    fn test_profile_format_default() {
        let format = ProfileFormat::default();
        assert_eq!(format, ProfileFormat::ChromeTrace);
    }
}
