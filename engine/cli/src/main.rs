use clap::{Parser, Subcommand};

mod commands;
mod templates;

#[derive(Parser)]
#[command(name = "silm")]
#[command(version, about = "Silmaril game engine CLI - code-first game development", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new game project
    New {
        /// Project name
        name: String,

        /// Template to use (basic, mmo, moba)
        #[arg(short, long, default_value = "basic")]
        template: String,

        /// Use local engine path dependencies (for engine development)
        #[arg(long)]
        local: bool,
    },

    /// Manage entity templates (levels, characters, props, UI, game state)
    Template {
        #[command(subcommand)]
        command: commands::template::TemplateCommand,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::New { name, template, local } => {
            commands::new::create_project(&name, &template, local)?;
        }
        Commands::Template { command } => {
            commands::template::handle_template_command(command)?;
        }
    }

    Ok(())
}
