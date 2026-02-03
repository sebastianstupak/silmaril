use anyhow::{bail, Result};
use clap::Subcommand;
use colored::Colorize;
use std::fs;
use std::path::PathBuf;

use crate::codegen::parser::{parse_query_components, to_snake_case};
use crate::codegen::system::{generate_system_code, SystemPhase};
use crate::codegen::validator::validate_snake_case;

#[derive(Subcommand)]
pub enum AddCommand {
    /// Add a new system
    System {
        /// System name in snake_case (e.g., health_regen, movement)
        name: String,

        /// Query components (e.g., "&mut Health,&RegenerationRate")
        #[arg(short, long)]
        query: String,

        /// Location: shared, client, server
        #[arg(short, long, default_value = "shared")]
        location: String,

        /// System phase: update, fixed_update, render
        #[arg(short, long, default_value = "update")]
        phase: String,

        /// Documentation string
        #[arg(long)]
        doc: Option<String>,
    },
}

pub fn handle_add_command(command: AddCommand) -> Result<()> {
    match command {
        AddCommand::System { name, query, location, phase, doc } => {
            add_system(&name, &query, &location, &phase, doc)
        }
    }
}

fn add_system(
    name: &str,
    query: &str,
    location: &str,
    phase_str: &str,
    doc: Option<String>,
) -> Result<()> {
    // Validate system name
    validate_snake_case(name)?;

    // Validate location
    if !["shared", "client", "server"].contains(&location) {
        bail!("Invalid location '{}'. Must be: shared, client, or server", location);
    }

    // Parse phase
    let phase = match phase_str {
        "update" => SystemPhase::Update,
        "fixed_update" => SystemPhase::FixedUpdate,
        "render" => SystemPhase::Render,
        _ => bail!("Invalid phase '{}'. Must be: update, fixed_update, or render", phase_str),
    };

    // Parse query components
    let components = parse_query_components(query)?;

    if components.is_empty() {
        bail!("Query must have at least one component");
    }

    // Generate code
    let code = generate_system_code(name, &components, phase, doc.clone());

    // Determine file path
    let systems_dir = PathBuf::from(location).join("src").join("systems");
    let file_path = systems_dir.join(format!("{}.rs", name));

    // Create directory if it doesn't exist
    if !systems_dir.exists() {
        fs::create_dir_all(&systems_dir)?;
    }

    // Check if file already exists
    if file_path.exists() {
        bail!("System file already exists: {}", file_path.display());
    }

    // Write file
    fs::write(&file_path, code)?;

    // Print success message
    println!("{}", "✅ System created successfully!".green().bold());
    println!();
    println!("{}", "📁 Files:".bold());
    println!("  ✓ {}", file_path.display());
    println!();
    println!("{}", "📝 Next steps:".bold());
    println!("  1. Review generated code");
    println!("  2. Implement system logic");
    println!("  3. Register in main.rs: app.add_system({})", name);
    println!("  4. Run tests: cargo test {}", name);
    println!();
    println!("{}", "⚠️  Manual steps required:".yellow().bold());
    println!("  - Add 'pub mod {};' to {}/src/systems/mod.rs", name, location);
    println!("  - Add 'pub use {}::{};' to export the system", name, name);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_system_name() {
        assert!(validate_snake_case("health_regen").is_ok());
        assert!(validate_snake_case("movement").is_ok());
        assert!(validate_snake_case("HealthRegen").is_err());
        assert!(validate_snake_case("health-regen").is_err());
    }

    #[test]
    fn test_parse_phase() {
        let phase = match "update" {
            "update" => SystemPhase::Update,
            "fixed_update" => SystemPhase::FixedUpdate,
            "render" => SystemPhase::Render,
            _ => panic!("Invalid phase"),
        };
        assert_eq!(phase, SystemPhase::Update);
    }
}
