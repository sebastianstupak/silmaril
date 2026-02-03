use anyhow::{Context, Result};
use colored::Colorize;
use std::fs;
use std::path::PathBuf;

use crate::templates::{BasicTemplate, Template};

pub fn create_project(name: &str, template: &str, use_local: bool) -> Result<()> {
    // Validate project name
    validate_project_name(name)?;

    // Get project path
    let project_path = PathBuf::from(name);
    if project_path.exists() {
        anyhow::bail!(
            "Directory '{}' already exists! Please choose a different name or remove the existing directory.",
            name
        );
    }

    println!(
        "{}",
        format!("🎮 Creating new Silmaril game project: {}", name).bright_blue().bold()
    );
    println!("   Template: {}", template.bright_cyan());
    if use_local {
        println!("   {}", "Using local engine dependencies (for engine development)".yellow());
    }
    println!();

    // Get template
    let template_impl: Box<dyn Template> = match template {
        "basic" => Box::new(BasicTemplate::new(name.to_string(), use_local)),
        "mmo" => anyhow::bail!("MMO template not yet implemented. Use 'basic' template for now."),
        "moba" => anyhow::bail!("MOBA template not yet implemented. Use 'basic' template for now."),
        _ => {
            anyhow::bail!("Unknown template: '{}'. Available templates: basic, mmo, moba", template)
        }
    };

    // Create project directory
    fs::create_dir(&project_path)
        .with_context(|| format!("Failed to create project directory: {}", name))?;

    // Create all template files
    let files = template_impl.files();
    let total_files = files.len();

    for (idx, file) in files.iter().enumerate() {
        let file_path = project_path.join(&file.path);

        // Create parent directories if needed
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {:?}", parent))?;
        }

        // Write file
        fs::write(&file_path, &file.content)
            .with_context(|| format!("Failed to write file: {:?}", file_path))?;

        println!(
            "   {} {}/{}  {}",
            "✓".bright_green(),
            idx + 1,
            total_files,
            file.path.bright_white()
        );
    }

    // Create empty directories
    let empty_dirs = ["assets", "assets/models", "assets/textures", "assets/audio"];
    for dir in &empty_dirs {
        let dir_path = project_path.join(dir);
        fs::create_dir_all(&dir_path)
            .with_context(|| format!("Failed to create directory: {}", dir))?;
    }

    println!();
    println!("{}", "✅ Project created successfully!".bright_green().bold());
    println!();
    print_next_steps(name, use_local);

    Ok(())
}

fn validate_project_name(name: &str) -> Result<()> {
    // Check if name is empty
    if name.is_empty() {
        anyhow::bail!("Project name cannot be empty");
    }

    // Check for invalid characters (only allow alphanumeric, dash, underscore)
    if !name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        anyhow::bail!(
            "Project name can only contain alphanumeric characters, dashes, and underscores.\nGot: '{}'",
            name
        );
    }

    // Check if name starts with a number (invalid for Rust packages)
    if name.chars().next().unwrap().is_numeric() {
        anyhow::bail!("Project name cannot start with a number");
    }

    // Check if name is a reserved Rust keyword
    let rust_keywords = [
        "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn",
        "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref",
        "return", "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe",
        "use", "where", "while", "async", "await", "dyn",
    ];

    if rust_keywords.contains(&name) {
        anyhow::bail!("Project name '{}' is a reserved Rust keyword", name);
    }

    Ok(())
}

fn print_next_steps(name: &str, use_local: bool) {
    println!("{}", "📋 Next steps:".bright_blue().bold());
    println!();
    println!("   1. Navigate to your project:");
    println!("      {}", format!("cd {}", name).bright_cyan());
    println!();

    if use_local {
        println!("   2. {} Verify local engine path in Cargo.toml", "⚠️".yellow());
        println!(
            "      {}",
            "Make sure the path points to your Silmaril engine directory".yellow()
        );
        println!();
    }

    println!("   {}. Start development:", if use_local { 3 } else { 2 });
    println!(
        "      {}  {}",
        "cargo xtask dev server".bright_cyan(),
        "(in one terminal)".dimmed()
    );
    println!(
        "      {}  {}",
        "cargo xtask dev client".bright_cyan(),
        "(in another terminal)".dimmed()
    );
    println!();

    println!("   {}. Add game features:", if use_local { 4 } else { 3 });
    println!(
        "      {}",
        "silm add component Health --shared --fields \"current:f32,max:f32\"".bright_cyan()
    );
    println!("      {}", "silm add system health_regen --shared".bright_cyan());
    println!();

    println!("   {}. Run tests:", if use_local { 5 } else { 4 });
    println!("      {}", "cargo xtask test all".bright_cyan());
    println!();

    println!("   {}. Build for release:", if use_local { 6 } else { 5 });
    println!("      {}", "cargo xtask build release".bright_cyan());
    println!();

    println!("{}", "📖 Documentation:".bright_blue().bold());
    println!("   • README.md in your project");
    println!("   • Run {} for available commands", "cargo xtask --help".bright_cyan());
    println!("   • Silmaril docs: https://github.com/your-org/silmaril");
    println!();

    println!("{}", "Happy game developing! 🎮".bright_green().bold());
}
