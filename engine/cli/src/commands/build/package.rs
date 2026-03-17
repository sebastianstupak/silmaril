//! Package helpers: zip filenames and Dockerfile generation.

// TODO(CLI.7): cargo-packager integration for AppImage/DMG/NSIS

/// Construct the zip filename for a platform distribution.
///
/// Format: `{name}-v{version}-{platform}.zip`
pub fn zip_filename(project_name: &str, version: &str, platform_name: &str) -> String {
    format!("{project_name}-v{version}-{platform_name}.zip")
}

/// Generate a minimal Dockerfile for a server binary.
///
/// Uses `debian:bookworm-slim` as the base, copies a `server` binary,
/// exposes UDP port 7777, and sets ENV lines for each provided entry.
pub fn generate_dockerfile(env_entries: &[(String, String)]) -> String {
    let mut lines = vec![
        "FROM debian:bookworm-slim".to_string(),
        String::new(),
        "COPY server /usr/local/bin/server".to_string(),
        "EXPOSE 7777/udp".to_string(),
    ];

    if !env_entries.is_empty() {
        lines.push(String::new());
        lines.push("# Override at runtime: docker run -e KEY=value ...".to_string());
        for (key, value) in env_entries {
            lines.push(format!("ENV {key}={value}"));
        }
    }

    lines.push(String::new());
    lines.push("ENTRYPOINT [\"/usr/local/bin/server\"]".to_string());
    lines.join("\n") + "\n"
}
