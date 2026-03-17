use anyhow::{bail, Result};
use clap::Subcommand;

pub mod component;
pub mod module;
pub mod system;
pub mod wiring;

use wiring::Target;

#[derive(Subcommand)]
pub enum AddCommand {
    /// Add a new ECS component to a domain slice
    Component {
        /// Component name in PascalCase (e.g., Health, PlayerState)
        name: String,

        /// Component fields (e.g., "current:f32,max:f32")
        #[arg(short, long)]
        fields: String,

        /// Domain name in snake_case (e.g., health, combat)
        #[arg(short, long)]
        domain: String,

        /// Target the shared crate
        #[arg(long, conflicts_with_all = ["server", "client"])]
        shared: bool,

        /// Target the server crate
        #[arg(long, conflicts_with_all = ["shared", "client"])]
        server: bool,

        /// Target the client crate
        #[arg(long, conflicts_with_all = ["shared", "server"])]
        client: bool,
    },

    /// Add a new ECS system to a domain slice
    System {
        /// System name in snake_case (e.g., health_regen, movement)
        name: String,

        /// Query components (e.g., "mut:Health,RegenerationRate")
        #[arg(short, long)]
        query: String,

        /// Domain name in snake_case (e.g., health, combat)
        #[arg(short, long)]
        domain: String,

        /// Target the shared crate
        #[arg(long, conflicts_with_all = ["server", "client"])]
        shared: bool,

        /// Target the server crate
        #[arg(long, conflicts_with_all = ["shared", "client"])]
        server: bool,

        /// Target the client crate
        #[arg(long, conflicts_with_all = ["shared", "server"])]
        client: bool,
    },
}

fn resolve_target(shared: bool, server: bool, client: bool) -> Result<Target> {
    match (shared, server, client) {
        (true, false, false) => Ok(Target::Shared),
        (false, true, false) => Ok(Target::Server),
        (false, false, true) => Ok(Target::Client),
        _ => bail!("must specify exactly one of --shared, --server, or --client"),
    }
}

pub fn handle_add_command(command: AddCommand) -> Result<()> {
    match command {
        AddCommand::Component { name, fields, domain, shared, server, client } => {
            let target = resolve_target(shared, server, client)?;
            component::add_component(&name, &fields, target, &domain)
        }
        AddCommand::System { name, query, domain, shared, server, client } => {
            let target = resolve_target(shared, server, client)?;
            system::add_system(&name, &query, target, &domain)
        }
    }
}
