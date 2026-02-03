//! Integration tests for the configuration system.
//!
//! These tests verify that the multi-source configuration system works correctly,
//! with the proper priority: env vars > config file > defaults.

#[cfg(feature = "config")]
use silmaril_profiling::{parse_duration, ProfileFormat, ProfilerConfig};
#[cfg(feature = "config")]
use std::io::Write;
#[cfg(feature = "config")]
use std::path::PathBuf;
#[cfg(feature = "config")]
use std::time::Duration;
#[cfg(feature = "config")]
use tempfile::NamedTempFile;

#[cfg(feature = "config")]
#[test]
fn test_config_priority_env_overrides_file() {
    // Create a config file
    let yaml_content = r#"
profiling:
  enabled: true
  persist: true
  output_dir: "file_dir"
  max_file_size_mb: 100
  format: chrome_trace

budgets:
  game_loop: 16.0ms
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(yaml_content.as_bytes()).unwrap();
    temp_file.flush().unwrap();

    // Set environment variable to override
    std::env::set_var("PROFILE_DIR", "env_dir");
    std::env::set_var("PROFILE_ENABLE", "false");

    // Load config from file
    let mut config = ProfilerConfig::from_file(temp_file.path()).unwrap();

    // Before merge_env, should have file values
    assert!(config.enabled);
    assert_eq!(config.output_dir, PathBuf::from("file_dir"));

    // Merge env overrides
    config.merge_env();

    // After merge_env, env vars should override
    assert!(!config.enabled); // Overridden by env
    assert_eq!(config.output_dir, PathBuf::from("env_dir")); // Overridden by env
    assert_eq!(config.max_file_size_mb, 100); // Still from file

    // Clean up
    std::env::remove_var("PROFILE_DIR");
    std::env::remove_var("PROFILE_ENABLE");
}

#[cfg(feature = "config")]
#[test]
fn test_full_config_loading_workflow() {
    let yaml_content = r#"
profiling:
  enabled: true
  persist: true
  output_dir: "profiling_output"
  max_file_size_mb: 150
  format: json
  retention:
    circular_buffer_frames: 2000
    save_on_budget_exceeded: false
    save_on_crash: true

budgets:
  custom_scope: 10.5ms
  another_scope: 2s
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(yaml_content.as_bytes()).unwrap();
    temp_file.flush().unwrap();

    let config = ProfilerConfig::from_file(temp_file.path()).unwrap();

    assert!(config.enabled);
    assert!(config.persist_to_disk);
    assert_eq!(config.output_dir, PathBuf::from("profiling_output"));
    assert_eq!(config.max_file_size_mb, 150);
    assert_eq!(config.format, ProfileFormat::Json);
    assert_eq!(config.retention.circular_buffer_frames, 2000);
    assert!(!config.retention.save_on_budget_exceeded);
    assert!(config.retention.save_on_crash);
    assert_eq!(config.budgets.get("custom_scope"), Some(&parse_duration("10.5ms").unwrap()));
    assert_eq!(config.budgets.get("another_scope"), Some(&Duration::from_secs(2)));
}

#[cfg(feature = "config")]
#[test]
fn test_env_only_configuration() {
    // Save old state
    let old_enable = std::env::var("PROFILE_ENABLE").ok();
    let old_persist = std::env::var("PROFILE_PERSIST").ok();
    let old_dir = std::env::var("PROFILE_DIR").ok();
    let old_budget = std::env::var("PROFILE_BUDGET_TEST").ok();

    // Set environment variables
    std::env::set_var("PROFILE_ENABLE", "true");
    std::env::set_var("PROFILE_PERSIST", "false");
    std::env::set_var("PROFILE_DIR", "env_only_dir");
    std::env::set_var("PROFILE_BUDGET_TEST", "5.5ms");

    let config = ProfilerConfig::from_env();

    assert!(config.enabled);
    assert!(!config.persist_to_disk);
    assert_eq!(config.output_dir, PathBuf::from("env_only_dir"));
    assert_eq!(config.budgets.get("test"), Some(&parse_duration("5.5ms").unwrap()));

    // Restore old state
    restore_or_remove("PROFILE_ENABLE", old_enable);
    restore_or_remove("PROFILE_PERSIST", old_persist);
    restore_or_remove("PROFILE_DIR", old_dir);
    restore_or_remove("PROFILE_BUDGET_TEST", old_budget);
}

#[cfg(feature = "config")]
#[test]
fn test_minimal_config_file() {
    // Test with a minimal config file that only specifies a few fields
    let yaml_content = r#"
profiling:
  enabled: false

budgets:
  minimal_scope: 1ms
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(yaml_content.as_bytes()).unwrap();
    temp_file.flush().unwrap();

    let config = ProfilerConfig::from_file(temp_file.path()).unwrap();

    // Specified fields
    assert!(!config.enabled);
    assert_eq!(config.budgets.get("minimal_scope"), Some(&Duration::from_millis(1)));

    // Default fields should be set
    assert!(config.persist_to_disk); // Defaults to true
    assert_eq!(config.output_dir, PathBuf::from("profiling_data")); // Default
    assert_eq!(config.max_file_size_mb, 100); // Default
}

#[cfg(feature = "config")]
#[test]
fn test_duration_parsing_variants() {
    // Test various duration formats
    assert_eq!(parse_duration("16ms").unwrap(), Duration::from_millis(16));
    assert_eq!(parse_duration("16.5ms").unwrap(), Duration::from_micros(16500));
    assert_eq!(parse_duration("1s").unwrap(), Duration::from_secs(1));
    assert_eq!(parse_duration("1.5s").unwrap(), Duration::from_micros(1_500_000));
    assert_eq!(parse_duration("500us").unwrap(), Duration::from_micros(500));
    assert_eq!(parse_duration("16").unwrap(), Duration::from_millis(16)); // No unit = ms
}

#[cfg(feature = "config")]
fn restore_or_remove(key: &str, old_value: Option<String>) {
    match old_value {
        Some(val) => std::env::set_var(key, val),
        None => std::env::remove_var(key),
    }
}
