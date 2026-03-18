//! Package helpers: zip filenames, Dockerfile generation, dist assembly, and zip creation.

// TODO(CLI.7): cargo-packager integration for AppImage/DMG/NSIS

use anyhow::Result;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tracing::{info, warn};
use walkdir::WalkDir;
use zip::write::SimpleFileOptions;
use zip::CompressionMethod;
use zip::ZipWriter;

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

/// Creates a zip archive from a directory.
///
/// Walks `source_dir` recursively using `walkdir::WalkDir`, adding each file
/// with its relative path. Directories are added as empty entries.
/// Uses `Deflated` compression.
pub fn create_zip(source_dir: &Path, zip_path: &Path) -> Result<()> {
    let file = fs::File::create(zip_path)
        .map_err(|e| anyhow::anyhow!("Failed to create zip file {}: {e}", zip_path.display()))?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

    for entry in WalkDir::new(source_dir) {
        let entry = entry?;
        let abs_path = entry.path();
        let relative = abs_path
            .strip_prefix(source_dir)
            .map_err(|e| anyhow::anyhow!("Failed to compute relative path: {e}"))?;

        // Skip the root directory itself
        if relative.as_os_str().is_empty() {
            continue;
        }

        // Use forward slashes for zip compatibility
        let name = relative
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Non-UTF-8 path: {}", relative.display()))?
            .replace('\\', "/");

        if entry.file_type().is_dir() {
            zip.add_directory(&name, options)?;
        } else {
            zip.start_file(&name, options)?;
            let mut f = fs::File::open(abs_path)?;
            let mut buf = Vec::new();
            f.read_to_end(&mut buf)?;
            zip.write_all(&buf)?;
        }
    }

    zip.finish()?;
    info!(zip = %zip_path.display(), "Created zip archive");
    Ok(())
}

/// Copy `assets/` from project root to dist dir if present.
///
/// Silently skips if the assets directory does not exist.
pub fn copy_assets(project_root: &Path, dist_platform_dir: &Path) -> Result<()> {
    let assets_src = project_root.join("assets");
    if !assets_src.is_dir() {
        return Ok(());
    }

    let assets_dst = dist_platform_dir.join("assets");
    copy_dir_recursive(&assets_src, &assets_dst)?;
    info!(src = %assets_src.display(), dst = %assets_dst.display(), "Copied assets");
    Ok(())
}

/// Recursively copy a directory tree.
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in WalkDir::new(src) {
        let entry = entry?;
        let abs_path = entry.path();
        let relative = abs_path.strip_prefix(src)?;
        let target = dst.join(relative);

        if entry.file_type().is_dir() {
            fs::create_dir_all(&target)?;
        } else {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(abs_path, &target)?;
        }
    }
    Ok(())
}

/// Assemble a native platform distribution directory.
///
/// Wipes and recreates `dist/<platform_name>/`, copies server and/or client
/// binaries from `target/[<triple>/]release/`, copies assets, and returns
/// the dist directory path.
pub fn assemble_native_dist(
    project_root: &Path,
    platform_name: &str,
    target_triple: Option<&str>,
    server_binary: bool,
    client_binary: bool,
    exe_extension: bool,
) -> Result<PathBuf> {
    let dist_dir = project_root.join("dist").join(platform_name);

    // Wipe and recreate
    if dist_dir.exists() {
        fs::remove_dir_all(&dist_dir)?;
    }
    fs::create_dir_all(&dist_dir)?;

    let release_dir = match target_triple {
        Some(triple) => project_root.join("target").join(triple).join("release"),
        None => project_root.join("target").join("release"),
    };

    let ext = if exe_extension { ".exe" } else { "" };

    if server_binary {
        let bin_name = format!("server{ext}");
        let src = release_dir.join(&bin_name);
        if src.is_file() {
            fs::copy(&src, dist_dir.join(&bin_name))?;
            info!(binary = %bin_name, platform = %platform_name, "Copied server binary");
        } else {
            warn!(path = %src.display(), "Server binary not found — skipping");
        }
    }

    if client_binary {
        let bin_name = format!("client{ext}");
        let src = release_dir.join(&bin_name);
        if src.is_file() {
            fs::copy(&src, dist_dir.join(&bin_name))?;
            info!(binary = %bin_name, platform = %platform_name, "Copied client binary");
        } else {
            warn!(path = %src.display(), "Client binary not found — skipping");
        }
    }

    copy_assets(project_root, &dist_dir)?;

    Ok(dist_dir)
}

/// Assemble a server distribution directory with a Dockerfile.
///
/// Wipes and recreates `dist/server/`, copies the server binary, generates
/// a Dockerfile with the given env entries, and returns the dist directory path.
pub fn assemble_server_dist(
    project_root: &Path,
    env_entries: &[(String, String)],
    exe_extension: bool,
) -> Result<PathBuf> {
    let dist_dir = project_root.join("dist").join("server");

    // Wipe and recreate
    if dist_dir.exists() {
        fs::remove_dir_all(&dist_dir)?;
    }
    fs::create_dir_all(&dist_dir)?;

    let ext = if exe_extension { ".exe" } else { "" };
    let bin_name = format!("server{ext}");
    let release_dir = project_root.join("target").join("release");
    let src = release_dir.join(&bin_name);

    if src.is_file() {
        fs::copy(&src, dist_dir.join(&bin_name))?;
        info!(binary = %bin_name, "Copied server binary");
    } else {
        warn!(path = %src.display(), "Server binary not found — skipping");
    }

    // Generate Dockerfile
    let dockerfile_content = generate_dockerfile(env_entries);
    fs::write(dist_dir.join("Dockerfile"), &dockerfile_content)?;
    info!("Generated Dockerfile in dist/server/");

    // No assets copy for server-only dist — server is headless
    Ok(dist_dir)
}
