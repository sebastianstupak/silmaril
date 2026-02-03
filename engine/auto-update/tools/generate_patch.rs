//! Tool for generating update patches and manifests.
//!
//! Usage:
//!   cargo run --bin generate_patch -- \
//!     --old-dir ./v1.0.0 \
//!     --new-dir ./v1.0.1 \
//!     --output ./patches \
//!     --version 1.0.1 \
//!     --channel stable

use chrono::Utc;
use clap::Parser;
use engine_auto_update::{
    manifest::{FileInfo, PatchInfo, UpdateManifest},
    patcher::create_patch,
    verifier::compute_file_hash,
    version::Version,
};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{info, warn, Level};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Old version directory
    #[clap(long)]
    old_dir: PathBuf,

    /// New version directory
    #[clap(long)]
    new_dir: PathBuf,

    /// Output directory for patches and manifest
    #[clap(long)]
    output: PathBuf,

    /// Version number (e.g., 1.0.1)
    #[clap(long)]
    version: String,

    /// Release channel
    #[clap(long, default_value = "stable")]
    channel: String,

    /// CDN base URL
    #[clap(long)]
    cdn_url: Option<String>,

    /// Changelog text
    #[clap(long, default_value = "")]
    changelog: String,

    /// File patterns to include (glob)
    #[clap(long, default_value = "**/*")]
    include: String,

    /// Minimum compatible version
    #[clap(long)]
    min_version: Option<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    let args = Args::parse();

    info!("Generating update patch");
    info!("  Old version: {}", args.old_dir.display());
    info!("  New version: {}", args.new_dir.display());
    info!("  Output: {}", args.output.display());
    info!("  Version: {}", args.version);
    info!("  Channel: {}", args.channel);

    // Parse version
    let version: Version = args.version.parse()?;

    // Create output directory
    fs::create_dir_all(&args.output)?;

    let patch_dir = args.output.join("patches");
    fs::create_dir_all(&patch_dir)?;

    // Find all files in new directory
    let new_files = find_files(&args.new_dir, &args.include)?;

    info!("Found {} files in new version", new_files.len());

    let mut manifest_files = Vec::new();

    for file_path in new_files {
        let rel_path = file_path.strip_prefix(&args.new_dir)?;
        let new_file = file_path.clone();
        let old_file = args.old_dir.join(rel_path);

        info!("Processing: {}", rel_path.display());

        // Compute hash of new file
        let hash = compute_file_hash(&new_file)?;
        let size = fs::metadata(&new_file)?.len();

        let cdn_url = args
            .cdn_url
            .as_ref()
            .map(|u| format!("{}/{}/{}", u, args.channel, rel_path.display()))
            .unwrap_or_else(|| {
                format!("https://cdn.example.com/{}/{}", args.channel, rel_path.display())
            });

        // Try to create patch if old file exists
        let patch_info = if old_file.exists() {
            let patch_file = patch_dir.join(format!("{}.patch", rel_path.display()));

            // Create parent directories for patch
            if let Some(parent) = patch_file.parent() {
                fs::create_dir_all(parent)?;
            }

            match create_patch(&old_file, &new_file, &patch_file) {
                Ok(()) => {
                    let patch_size = fs::metadata(&patch_file)?.len();
                    let patch_hash = compute_file_hash(&patch_file)?;

                    info!(
                        "  Created patch: {} bytes ({}% of full file)",
                        patch_size,
                        (patch_size as f64 / size as f64 * 100.0) as u32
                    );

                    let patch_cdn_url = args
                        .cdn_url
                        .as_ref()
                        .map(|u| {
                            format!("{}/{}/patches/{}.patch", u, args.channel, rel_path.display())
                        })
                        .unwrap_or_else(|| {
                            format!(
                                "https://cdn.example.com/{}/patches/{}.patch",
                                args.channel,
                                rel_path.display()
                            )
                        });

                    Some(PatchInfo {
                        url: patch_cdn_url,
                        sha256: patch_hash,
                        size: patch_size,
                        from_version: version, // Assuming patches are from previous version
                    })
                }
                Err(e) => {
                    warn!("  Failed to create patch: {}", e);
                    None
                }
            }
        } else {
            info!("  New file (no patch)");
            None
        };

        manifest_files.push(FileInfo {
            path: rel_path.display().to_string(),
            sha256: hash,
            size,
            url: cdn_url,
            patch: patch_info,
        });
    }

    // Create manifest
    let min_version = args.min_version.map(|v| v.parse()).transpose()?;

    let manifest = UpdateManifest {
        version,
        release_date: Utc::now(),
        changelog: args.changelog,
        files: manifest_files,
        signature: None, // Signature should be added separately with signing tool
        channel: args.channel,
        min_version,
    };

    // Validate manifest
    manifest.validate()?;

    // Write manifest
    let manifest_path = args.output.join("manifest.json");
    let manifest_json = serde_json::to_string_pretty(&manifest)?;
    fs::write(&manifest_path, manifest_json)?;

    info!("");
    info!("Update patch generated successfully!");
    info!("  Manifest: {}", manifest_path.display());
    info!("  Total files: {}", manifest.files.len());
    info!(
        "  Total download size (with patches): {} bytes",
        manifest.total_download_size(None)
    );

    info!("");
    info!("Next steps:");
    info!("  1. Sign the manifest with your private key");
    info!("  2. Upload patches to CDN: {}", patch_dir.display());
    info!("  3. Upload manifest to update server");

    Ok(())
}

fn find_files(dir: &Path, _pattern: &str) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let mut files = Vec::new();

    fn visit_dir(dir: &Path, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
        if dir.is_dir() {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    visit_dir(&path, files)?;
                } else {
                    files.push(path);
                }
            }
        }
        Ok(())
    }

    visit_dir(dir, &mut files)?;

    // TODO: Apply glob pattern filtering
    // For now, include all files

    Ok(files)
}
