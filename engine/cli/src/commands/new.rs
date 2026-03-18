// NOTE: Project creation is now delegated to engine_ops::project.
// The CLI layer handles user-facing output (colored messages, next-steps).

use anyhow::Result;
use colored::Colorize;

pub fn create_project(name: &str, template: &str, use_local: bool) -> Result<()> {
    println!(
        "{}",
        format!("🎮 Creating new Silmaril game project: {}", name).bright_blue().bold()
    );
    println!("   Template: {}", template.bright_cyan());
    if use_local {
        println!("   {}", "Using local engine dependencies (for engine development)".yellow());
    }
    println!();

    // Delegate to shared ops layer
    let file_count = engine_ops::project::create_project(name, template, use_local)?;

    // CLI-specific progress output
    println!(
        "   {} {}/{} files written",
        "✓".bright_green(),
        file_count,
        file_count,
    );

    println!();
    println!("{}", "✅ Project created successfully!".bright_green().bold());
    println!();
    print_next_steps(name, use_local);

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
