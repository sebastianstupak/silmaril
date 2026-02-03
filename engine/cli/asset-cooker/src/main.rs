//! Asset Cooker CLI tool
//!
//! Convert raw assets → optimized cooked assets → bundles
//!
//! # Commands
//!
//! - `cook <source_dir> <output_dir>` - Process raw assets → cooked
//! - `bundle <manifest> <output_bundle>` - Pack manifest → bundle file
//! - `validate <asset_path>` - Validate asset format
//! - `info <asset_path>` - Display asset metadata
//! - `generate <type> <params>` - Generate procedural asset

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing::info;

mod bundle;
mod cook;
mod generate;
mod info;
mod validate;

#[derive(Parser)]
#[command(name = "asset-cooker")]
#[command(about = "Asset pipeline tool for cooking, bundling, and validating assets")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Cook raw assets into optimized format
    Cook {
        /// Source directory containing raw assets
        source_dir: PathBuf,

        /// Output directory for cooked assets
        output_dir: PathBuf,

        /// Generate mipmaps for textures
        #[arg(long)]
        generate_mipmaps: bool,

        /// Optimize meshes (vertex cache, overdraw)
        #[arg(long)]
        optimize_meshes: bool,

        /// Process assets recursively
        #[arg(short, long)]
        recursive: bool,
    },

    /// Create asset bundle from manifest
    Bundle {
        /// Path to manifest file (YAML)
        manifest: PathBuf,

        /// Output bundle file path
        output: PathBuf,

        /// Compression format
        #[arg(long, default_value = "none")]
        compression: String,
    },

    /// Validate asset file
    Validate {
        /// Path to asset file
        asset_path: PathBuf,
    },

    /// Display asset metadata
    Info {
        /// Path to asset file
        asset_path: PathBuf,
    },

    /// Generate procedural asset
    Generate {
        /// Asset type (mesh, texture, audio)
        asset_type: String,

        /// Asset parameters (type-specific)
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        params: Vec<String>,

        /// Output file path
        #[arg(short, long)]
        output: PathBuf,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    let log_level = if cli.verbose { "debug" } else { "info" };
    tracing_subscriber::fmt().with_env_filter(log_level).init();

    info!("Asset Cooker v{}", env!("CARGO_PKG_VERSION"));

    match cli.command {
        Commands::Cook { source_dir, output_dir, generate_mipmaps, optimize_meshes, recursive } => {
            cook::run(source_dir, output_dir, generate_mipmaps, optimize_meshes, recursive)?;
        }
        Commands::Bundle { manifest, output, compression } => {
            bundle::run(manifest, output, &compression)?;
        }
        Commands::Validate { asset_path } => {
            validate::run(asset_path)?;
        }
        Commands::Info { asset_path } => {
            info::run(asset_path)?;
        }
        Commands::Generate { asset_type, params, output } => {
            generate::run(&asset_type, params, output)?;
        }
    }

    info!("Done");
    Ok(())
}
