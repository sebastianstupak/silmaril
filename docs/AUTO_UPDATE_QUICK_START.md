# Auto-Update Quick Start Guide

## Installation

Add to your game's `Cargo.toml`:

```toml
[dependencies]
engine-auto-update = { path = "../engine/auto-update" }
tokio = { version = "1.35", features = ["full"] }
```

## Basic Usage

### 1. Initialize Update Manager

```rust
use engine_auto_update::{UpdateConfig, UpdateManager, UpdateVersion as Version};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure update system
    let config = UpdateConfig::new(
        "https://updates.yourgame.com/manifest.json".to_string(),
        Version::new(1, 0, 0),  // Current version
        PathBuf::from("./game"), // Installation directory
    );

    let mut manager = UpdateManager::new(config)?;

    // Check for updates
    check_and_install_updates(&mut manager).await?;

    Ok(())
}
```

### 2. Check for Updates

```rust
async fn check_and_install_updates(manager: &mut UpdateManager) -> Result<(), Box<dyn std::error::Error>> {
    println!("Checking for updates...");

    match manager.check_for_updates().await? {
        Some(manifest) => {
            println!("Update available: {}", manifest.version);
            println!("Changelog:\n{}", manifest.changelog);

            // Download and install
            manager.download_update(&manifest).await?;
            manager.install_update().await?;

            println!("Update installed successfully!");
        }
        None => {
            println!("Already up to date!");
        }
    }

    Ok(())
}
```

### 3. With Progress Tracking

```rust
use std::time::Duration;

async fn download_with_progress(
    manager: &mut UpdateManager,
    manifest: &UpdateManifest,
) -> Result<(), Box<dyn std::error::Error>> {
    let progress = manager.get_progress_tracker();

    // Start download in background
    let download_handle = {
        let mut manager = manager.clone();
        let manifest = manifest.clone();
        tokio::spawn(async move {
            manager.download_update(&manifest).await
        })
    };

    // Monitor progress
    while !download_handle.is_finished() {
        let p = progress.get_progress();
        print!("\rProgress: {:.1}% - {} - ETA: {}   ",
            p.percentage(),
            p.speed_string(),
            p.eta_string()
        );
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    println!("\nDownload complete!");
    download_handle.await??;
    Ok(())
}
```

## Advanced Features

### Rollback

```rust
// Automatic rollback on failure
if let Err(e) = manager.install_update().await {
    eprintln!("Update failed: {}", e);
    // System automatically rolled back
}

// Manual rollback
manager.rollback()?;
```

### Channel Management

```rust
use engine_auto_update::UpdateChannel;

// Switch to beta channel
manager.switch_channel(UpdateChannel::Beta)?;

// Check for beta updates
if let Some(manifest) = manager.check_for_updates().await? {
    println!("Beta update available: {}", manifest.version);
}
```

### Custom Configuration

```rust
use engine_auto_update::downloader::DownloadConfig;
use std::time::Duration;

let mut config = UpdateConfig::new(...);

// Configure download behavior
config.download_config = DownloadConfig {
    max_speed: 1_000_000,  // 1 MB/s bandwidth limit
    timeout: Duration::from_secs(60),
    max_retries: 5,
    chunk_size: 16384,
};

// Enable signature verification
config.public_key = Some("your_ed25519_public_key_hex".to_string());

// Disable auto-check
config.auto_check = false;
```

## Building Updates

### 1. Generate Patch

```bash
cargo run --bin generate_patch -- \
    --old-dir ./previous-version \
    --new-dir ./new-version \
    --output ./updates \
    --version 1.0.1 \
    --channel stable \
    --cdn-url https://cdn.yourgame.com \
    --changelog "Bug fixes and improvements"
```

### 2. Sign Manifest (Optional but Recommended)

```python
# sign_manifest.py
import sys
import json
from cryptography.hazmat.primitives.asymmetric import ed25519
from cryptography.hazmat.primitives import serialization

# Load private key
with open('private.key', 'rb') as f:
    private_key = serialization.load_pem_private_key(f.read(), password=None)

# Load manifest
with open('updates/manifest.json', 'r') as f:
    manifest = json.load(f)

# Sign
manifest_bytes = json.dumps(manifest, sort_keys=True).encode()
signature = private_key.sign(manifest_bytes)

# Add signature
manifest['signature'] = signature.hex()

# Save
with open('updates/manifest.json', 'w') as f:
    json.dump(manifest, f, indent=2)

print(f"Manifest signed successfully")
```

### 3. Upload to CDN

```bash
# AWS S3 + CloudFront
aws s3 sync updates/ s3://your-game-updates/stable/1.0.1/
aws cloudfront create-invalidation --distribution-id E123456 --paths "/*"

# Or Cloudflare R2
wrangler r2 object put game-updates/stable/1.0.1/manifest.json --file=updates/manifest.json
```

## Testing

### Unit Tests

```bash
cd engine/auto-update
cargo test
```

### Integration Tests

```bash
cargo test --test integration_test
```

### Benchmarks

```bash
cargo bench
```

## Troubleshooting

### Update Fails with "Network Error"

```rust
// Increase timeout
config.download_config.timeout = Duration::from_secs(120);

// Increase retries
config.download_config.max_retries = 5;
```

### Signature Verification Fails

```rust
// Check public key format (should be hex-encoded)
config.public_key = Some("a1b2c3d4...".to_string());

// Ensure manifest was signed correctly
// Verify with: python3 verify_signature.py
```

### Disk Space Issues

```rust
// Check available space before update
let manifest = manager.check_for_updates().await?.unwrap();
let required_space = manifest.total_download_size(Some(manager.current_version()));

if available_disk_space() < required_space * 2 {
    eprintln!("Insufficient disk space");
    return Err(...);
}
```

## Production Checklist

Before deploying:

- [ ] Test rollback functionality
- [ ] Enable signature verification
- [ ] Set up CDN infrastructure
- [ ] Configure monitoring/alerting
- [ ] Test on slow connections
- [ ] Test disk full scenarios
- [ ] Document disaster recovery procedures
- [ ] Set appropriate cache TTLs
- [ ] Test with production-sized updates
- [ ] Enable HTTPS enforcement

## See Also

- [AUTO_UPDATE_IMPLEMENTATION_SUMMARY.md](AUTO_UPDATE_IMPLEMENTATION_SUMMARY.md) - Full system documentation
- [AUTO_UPDATE_CDN_SETUP.md](AUTO_UPDATE_CDN_SETUP.md) - CDN configuration guide
- [AUTO_UPDATE_RELIABILITY_REPORT.md](AUTO_UPDATE_RELIABILITY_REPORT.md) - Failure scenarios and recovery
- [AUTO_UPDATE_PERFORMANCE_REPORT.md](AUTO_UPDATE_PERFORMANCE_REPORT.md) - Performance analysis

## Support

For issues or questions:
1. Check the comprehensive documentation
2. Review integration tests for examples
3. Enable debug logging: `RUST_LOG=engine_auto_update=debug`
