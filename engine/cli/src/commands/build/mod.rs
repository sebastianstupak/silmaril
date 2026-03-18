//! Build command -- thin CLI wrapper over engine_ops::build.
//!
//! The CLI owns clap structs, spinners (indicatif), and --watch mode.
//! All build logic is delegated to engine_ops::build.

#[allow(unused_imports)]
pub use engine_ops::build::{
    build_all_platforms, check_docker, check_tool, dist_dir_name, host_target_triple,
    parse_dev_section, parse_project_name, parse_project_version, platform_from_str, BuildKind,
    BuildRunner, BuildTool, Platform, RealRunner, KNOWN_PLATFORMS,
};
#[allow(unused_imports)]
pub use engine_ops::build::{env, installer, native, package, wasm};

use anyhow::{bail, Result};
use clap::Args;
use engine_ops::ProgressSink;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;
use tracing::{info, warn};

#[derive(Args, Debug)]
pub struct BuildCommand {
    #[arg(long, num_args = 1..)]
    pub platform: Option<Vec<String>>,
    #[arg(long)]
    pub release: bool,
    #[arg(long)]
    pub watch: bool,
    #[arg(long)]
    pub env_file: Option<String>,
}

#[derive(Args, Debug)]
pub struct PackageCommand {
    #[arg(long, num_args = 1..)]
    pub platform: Option<Vec<String>>,
    #[arg(long)]
    pub out_dir: Option<String>,
    #[arg(long)]
    pub installer: bool,
}

struct SpinnerProgress<'a> { spinner: &'a ProgressBar }
impl ProgressSink for SpinnerProgress<'_> {
    fn on_start(&self, _: &str, _: usize) {}
    fn on_step(&self, _: &str, _: usize, msg: &str) { self.spinner.set_message(msg.to_string()); }
    fn on_done(&self, _: &str, _: bool) {}
}

fn make_spinner(message: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::with_template("{spinner:.cyan} {msg}").unwrap()
        .tick_strings(&["\u{280b}","\u{2819}","\u{2839}","\u{2838}","\u{283c}","\u{2834}","\u{2826}","\u{2827}","\u{2807}","\u{280f}","\u{2713}"]));
    pb.set_message(message.to_string());
    pb.enable_steady_tick(std::time::Duration::from_millis(80));
    pb
}

pub fn handle_build_command(cmd: BuildCommand, project_root: PathBuf) -> Result<()> {
    let game_toml_path = project_root.join("game.toml");
    let game_toml_content = std::fs::read_to_string(&game_toml_path)
        .map_err(|e| anyhow::anyhow!("Failed to read game.toml: {e}"))?;
    let platform_names: Vec<String> = if let Some(ref platforms) = cmd.platform {
        platforms.clone()
    } else {
        env::parse_build_section(&game_toml_content)
            .ok_or_else(|| anyhow::anyhow!("no platforms specified -- add [build] platforms = [...] to game.toml, or use --platform <name>"))?
    };
    let spinner = make_spinner(&format!("building {} platform(s)...", platform_names.len()));
    let env_file_path = cmd.env_file.as_ref().map(PathBuf::from);
    let progress = SpinnerProgress { spinner: &spinner };
    let result = build_all_platforms(&RealRunner, &project_root, &game_toml_content, &platform_names, cmd.release, env_file_path.as_deref(), false, &progress);
    match &result {
        Ok(()) => spinner.finish_with_message(format!("built {} platform(s)", platform_names.len())),
        Err(e) => spinner.finish_with_message(format!("build failed: {e}")),
    }
    if !cmd.watch { return result; }
    result.ok();
    info!("[silm] watching for changes... (Ctrl+C to stop)");
    use notify_debouncer_full::{new_debouncer, notify::{RecursiveMode, Watcher}};
    use std::sync::mpsc;
    use std::time::Duration;
    let (tx, rx) = mpsc::channel();
    let mut debouncer = new_debouncer(Duration::from_millis(500), None, tx)
        .map_err(|e| anyhow::anyhow!("failed to start file watcher: {e}"))?;
    for dir in &["shared", "server", "client", "assets"] {
        let watch_dir = project_root.join(dir);
        if watch_dir.is_dir() { debouncer.watcher().watch(&watch_dir, RecursiveMode::Recursive).ok(); }
    }
    loop {
        match rx.recv() {
            Ok(Ok(_events)) => {
                let spinner = make_spinner("rebuilding...");
                let game_toml_content = std::fs::read_to_string(&game_toml_path).unwrap_or_default();
                let env_file_path = cmd.env_file.as_ref().map(PathBuf::from);
                let progress = SpinnerProgress { spinner: &spinner };
                let result = build_all_platforms(&RealRunner, &project_root, &game_toml_content, &platform_names, cmd.release, env_file_path.as_deref(), false, &progress);
                match &result {
                    Ok(()) => spinner.finish_with_message("rebuild complete"),
                    Err(e) => spinner.finish_with_message(format!("rebuild failed: {e}")),
                }
            }
            Ok(Err(errors)) => { for e in errors { warn!(error = ?e, "watch error"); } }
            Err(_) => break,
        }
    }
    Ok(())
}

pub fn handle_package_command(cmd: PackageCommand, project_root: PathBuf) -> Result<()> {
    let game_toml_path = project_root.join("game.toml");
    let game_toml_content = std::fs::read_to_string(&game_toml_path)
        .map_err(|e| anyhow::anyhow!("Failed to read game.toml: {e}"))?;
    let project_name = parse_project_name(&game_toml_content)
        .ok_or_else(|| anyhow::anyhow!("game.toml is missing [project] name"))?;
    let version = parse_project_version(&game_toml_content);
    let platform_names: Vec<String> = if let Some(ref platforms) = cmd.platform {
        platforms.clone()
    } else {
        env::parse_build_section(&game_toml_content)
            .ok_or_else(|| anyhow::anyhow!("no platforms specified -- add [build] platforms = [...] to game.toml, or use --platform <name>"))?
    };
    let build_cmd = BuildCommand { platform: cmd.platform.clone(), release: true, env_file: None, watch: false };
    handle_build_command(build_cmd, project_root.clone())?;
    let out_dir = cmd.out_dir.as_ref().map(PathBuf::from).unwrap_or_else(|| project_root.clone());
    let build_env = env::parse_build_env(&game_toml_content);
    let spinner = make_spinner(&format!("packaging {} platform(s)...", platform_names.len()));
    for name in &platform_names {
        let platform = platform_from_str(name)?;
        spinner.set_message(format!("packaging {}...", name));
        let dist_dir = if name == "wasm" {
            let wasm_dist = project_root.join("dist").join("wasm");
            if !wasm_dist.is_dir() { bail!("dist/wasm/ not found -- did the WASM build succeed?"); }
            wasm_dist
        } else if name == "server" {
            package::assemble_server_dist(&project_root, &build_env, platform.uses_exe_extension())?
        } else {
            let (server_bin, client_bin) = match platform.build_kind() {
                BuildKind::ServerAndClient => (true, true),
                BuildKind::ServerOnly => (true, false),
                BuildKind::ClientOnly => (false, true),
            };
            let target_triple = if platform.target_triple() != host_target_triple() { Some(platform.target_triple()) } else { None };
            package::assemble_native_dist(&project_root, name, target_triple, server_bin, client_bin, platform.uses_exe_extension())?
        };
        let zip_name = package::zip_filename(&project_name, &version, name);
        let zip_path = out_dir.join(&zip_name);
        package::create_zip(&dist_dir, &zip_path)?;
        info!(zip = %zip_name, "Package complete");
    }
    spinner.finish_with_message(format!("packaged {} platform(s)", platform_names.len()));
    if cmd.installer {
        match installer::check_packager() {
            Ok(()) => {
                let description = "A game built with Silmaril";
                let config = installer::generate_packager_config(&project_name, &version, description, "client");
                std::fs::write(project_root.join("packager.toml"), &config)?;
                installer::run_packager(&project_root)?;
            }
            Err(e) => { info!("{}", e); info!("[silm] skipping installer generation"); }
        }
    }
    Ok(())
}
