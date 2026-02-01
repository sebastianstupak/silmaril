# Engine Auto-Update

## Purpose
The auto-update crate provides seamless game updates:
- **Update Detection**: Check for new versions from update server
- **Delta Patching**: Download only changed files to minimize bandwidth
- **Background Download**: Download updates while game is running
- **Safe Installation**: Atomic updates with rollback on failure
- **Version Management**: Track and manage multiple game versions

## MUST READ Documentation
Before working on this crate, read these documents in order:

1. **[phase4-auto-update.md](../../docs/phase4-auto-update.md)** - Auto-update system design

## Related Crates
- **engine-networking**: Uses network stack for update downloads
- **engine-core**: Platform abstraction for file operations

## Quick Example
```rust
use engine_auto_update::{UpdateManager, UpdateStatus};

fn check_for_updates(updater: &mut UpdateManager) {
    match updater.check_for_updates() {
        UpdateStatus::Available(version) => {
            println!("New version available: {}", version);
            updater.download_update();
        }
        UpdateStatus::UpToDate => {
            println!("Game is up to date");
        }
    }
}

fn apply_update(updater: &mut UpdateManager) {
    // Apply update and restart
    updater.apply_update_and_restart()?;
}
```

## Key Dependencies
- `reqwest` - HTTP client for downloads
- `bsdiff` - Delta patching
- `engine-core` - Platform abstraction

## Performance Targets
- Delta patches: <10MB for typical updates
- Download speed: Limited by network, not CPU
- Update installation: <10 seconds
- Zero downtime updates (background download)
