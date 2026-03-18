use clap::{Parser, Subcommand};

mod codegen;
mod commands;
// TODO: templates module is superseded by engine_ops::project.
// Kept during incremental migration; remove once all commands use engine_ops.
#[allow(dead_code, unused_imports)]
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

    /// Add components and systems
    Add {
        #[command(subcommand)]
        command: commands::add::AddCommand,
    },

    /// Start the hot-reload development server (server + client, or either)
    Dev {
        #[command(subcommand)]
        subcmd: Option<commands::dev::DevSubcommand>,
    },

    /// Manage installed game modules
    Module {
        #[command(subcommand)]
        command: commands::module::ModuleCommand,
    },

    /// Build the game for one or more target platforms
    Build {
        #[command(flatten)]
        command: commands::build::BuildCommand,
    },

    /// Package the game into distributable zip archives
    Package {
        #[command(flatten)]
        command: commands::build::PackageCommand,
    },

    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::New { name, template, local } => {
            commands::new::create_project(&name, &template, local)?;
        }
        Commands::Template { command } => {
            commands::template::handle_template_command(command)?;
        }
        Commands::Add { command } => {
            commands::add::handle_add_command(command)?;
        }
        Commands::Dev { subcmd } => {
            commands::dev::handle_dev_command(subcmd).await?;
        }
        Commands::Module { command } => {
            let cwd = std::env::current_dir()?;
            let project_root = commands::add::wiring::find_project_root(&cwd)?;
            commands::module::handle_module_command(command, project_root)?;
        }
        Commands::Build { command } => {
            let cwd = std::env::current_dir()?;
            let project_root = commands::add::wiring::find_project_root(&cwd)?;
            commands::build::handle_build_command(command, project_root)?;
        }
        Commands::Package { command } => {
            let cwd = std::env::current_dir()?;
            let project_root = commands::add::wiring::find_project_root(&cwd)?;
            commands::build::handle_package_command(command, project_root)?;
        }
        Commands::Completions { shell } => {
            let mut cmd = <Cli as clap::CommandFactory>::command();
            commands::completions::generate_completions(shell, &mut cmd)?;
        }
    }

    Ok(())
}
