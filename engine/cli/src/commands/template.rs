use anyhow::{Context, Result};
use clap::Subcommand;
use colored::Colorize;
use std::path::{Path, PathBuf};

#[derive(Subcommand)]
pub enum TemplateCommand {
    /// Create a new template
    Add {
        /// Template name (without extension)
        name: String,

        /// Template type
        #[arg(short = 't', long = "type", value_parser = parse_template_type)]
        template_type: TemplateType,

        /// Optional description
        #[arg(short, long)]
        description: Option<String>,

        /// Optional author name
        #[arg(short, long)]
        author: Option<String>,
    },

    /// Validate a template file
    Validate {
        /// Path to the template file to validate
        path: PathBuf,
    },

    /// Compile template to binary format
    Compile {
        /// Path to the template file or directory to compile
        path: PathBuf,

        /// Output path for compiled template(s)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Compile all templates in directory recursively
        #[arg(long)]
        all: bool,

        /// Watch for changes and recompile automatically
        #[arg(long)]
        watch: bool,
    },

    /// List all templates
    List {
        /// Base path to search for templates (default: assets/templates)
        base_path: Option<PathBuf>,
    },

    /// Show template entity hierarchy
    Tree {
        /// Path to the template file
        path: PathBuf,
    },

    /// Rename a template
    Rename {
        /// Path to the template file
        path: PathBuf,

        /// New name (without extension)
        new_name: String,
    },

    /// Delete a template
    Delete {
        /// Path to the template file
        path: PathBuf,

        /// Skip confirmation prompt
        #[arg(short, long)]
        yes: bool,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum TemplateType {
    Level,
    Character,
    Prop,
    UI,
    GameState,
}

impl std::fmt::Display for TemplateType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Level => write!(f, "level"),
            Self::Character => write!(f, "character"),
            Self::Prop => write!(f, "prop"),
            Self::UI => write!(f, "ui"),
            Self::GameState => write!(f, "game_state"),
        }
    }
}

fn parse_template_type(s: &str) -> Result<TemplateType, String> {
    match s.to_lowercase().as_str() {
        "level" => Ok(TemplateType::Level),
        "character" => Ok(TemplateType::Character),
        "prop" => Ok(TemplateType::Prop),
        "ui" => Ok(TemplateType::UI),
        "game_state" | "gamestate" => Ok(TemplateType::GameState),
        _ => Err(format!(
            "Invalid template type: '{}'. Valid types: level, character, prop, ui, game_state",
            s
        )),
    }
}

pub fn handle_template_command(cmd: TemplateCommand) -> Result<()> {
    match cmd {
        TemplateCommand::Add { name, template_type, description, author } => {
            add_template(&name, template_type, description, author)
        }
        TemplateCommand::Validate { path } => validate_template(&path),
        TemplateCommand::Compile { path, output, all, watch } => {
            compile_template(&path, output, all, watch)
        }
        TemplateCommand::List { base_path } => list_templates(base_path),
        TemplateCommand::Tree { path } => show_template_tree(&path),
        TemplateCommand::Rename { path, new_name } => rename_template(&path, &new_name),
        TemplateCommand::Delete { path, yes } => delete_template(&path, yes),
    }
}

fn add_template(
    name: &str,
    template_type: TemplateType,
    description: Option<String>,
    author: Option<String>,
) -> Result<()> {
    println!("{}", format!("Creating template: {}", name).bright_blue().bold());
    println!("   Type: {}", template_type.to_string().bright_cyan());
    if let Some(ref desc) = description {
        println!("   Description: {}", desc.dimmed());
    }
    if let Some(ref auth) = author {
        println!("   Author: {}", auth.dimmed());
    }
    println!();

    // Determine base path and subdirectory based on template type
    let base_path = PathBuf::from("assets/templates");
    let type_dir = match template_type {
        TemplateType::Level => "levels",
        TemplateType::Character => "characters",
        TemplateType::Prop => "props",
        TemplateType::UI => "ui",
        TemplateType::GameState => "game_state",
    };

    // TODO: Once engine_templating operations module is implemented, replace this with:
    // use engine_templating::operations::{create_template, CreateTemplateOptions};
    //
    // let options = CreateTemplateOptions {
    //     name: name.to_string(),
    //     template_type: match template_type {
    //         TemplateType::Level => engine_templating::operations::TemplateType::Level,
    //         TemplateType::Character => engine_templating::operations::TemplateType::Character,
    //         TemplateType::Prop => engine_templating::operations::TemplateType::Prop,
    //         TemplateType::UI => engine_templating::operations::TemplateType::UI,
    //         TemplateType::GameState => engine_templating::operations::TemplateType::GameState,
    //     },
    //     description,
    //     author,
    // };
    //
    // let template_path = create_template(&base_path, options)
    //     .with_context(|| format!("Failed to create template: {}", name))?;

    // Temporary stub implementation
    let template_dir = base_path.join(type_dir);
    std::fs::create_dir_all(&template_dir)
        .with_context(|| format!("Failed to create directory: {:?}", template_dir))?;

    let template_path = template_dir.join(format!("{}.yaml", name));

    if template_path.exists() {
        anyhow::bail!(
            "Template already exists: {}\nUse a different name or delete the existing template.",
            template_path.display()
        );
    }

    // Create a basic template YAML structure
    let yaml_content = create_template_yaml_stub(name, description.as_deref(), author.as_deref());

    std::fs::write(&template_path, yaml_content)
        .with_context(|| format!("Failed to write template file: {:?}", template_path))?;

    println!(
        "{} {}",
        "✓".bright_green(),
        format!("Created: {}", template_path.display()).bright_white()
    );
    println!();
    println!("{}", "Next steps:".bright_blue().bold());
    println!("   1. Edit the template: {}", template_path.display());
    println!(
        "   2. Validate: {}",
        format!("silm template validate {}", template_path.display()).bright_cyan()
    );
    println!();

    Ok(())
}

fn create_template_yaml_stub(
    name: &str,
    description: Option<&str>,
    author: Option<&str>,
) -> String {
    let desc = description.unwrap_or("TODO: Add description");
    let auth = author.unwrap_or("TODO: Add author");

    format!(
        r#"metadata:
  name: "{}"
  description: "{}"
  author: "{}"
  version: "1.0"

entities:
  Root:
    components:
      Transform:
        position: [0, 0, 0]
        rotation: [0, 0, 0, 1]
        scale: [1, 1, 1]
      tags: []

    # Add child entities here
    # children:
    #   ChildName:
    #     components:
    #       Transform:
    #         position: [0, 0, 0]
"#,
        name, desc, auth
    )
}

fn validate_template(path: &Path) -> Result<()> {
    println!("{}", format!("Validating template: {}", path.display()).bright_blue().bold());
    println!();

    // TODO: Once engine_templating operations module is implemented, replace this with:
    // use engine_templating::operations::validate_template;
    //
    // let report = validate_template(path)
    //     .with_context(|| format!("Failed to validate template: {}", path.display()))?;
    //
    // if report.is_valid {
    //     println!("{}", "✓ Template is valid!".bright_green().bold());
    //     println!();
    //     println!("   Entities: {}", report.entity_count);
    //     if !report.template_references.is_empty() {
    //         println!("   References: {}", report.template_references.len());
    //         for reference in &report.template_references {
    //             println!("      - {}", reference.dimmed());
    //         }
    //     }
    //     if !report.warnings.is_empty() {
    //         println!();
    //         println!("{}", "Warnings:".yellow().bold());
    //         for warning in &report.warnings {
    //             println!("   {} {}", "⚠".yellow(), warning);
    //         }
    //     }
    // } else {
    //     println!("{}", "✗ Template validation failed!".bright_red().bold());
    //     println!();
    //     println!("{}", "Errors:".bright_red().bold());
    //     for error in &report.errors {
    //         println!("   {} {}", "✗".bright_red(), error);
    //     }
    //     anyhow::bail!("Template validation failed");
    // }

    // Temporary stub implementation
    if !path.exists() {
        anyhow::bail!("Template file not found: {}", path.display());
    }

    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read template file: {}", path.display()))?;

    // Basic YAML validation
    let _parsed: serde_yaml::Value = serde_yaml::from_str(&content)
        .with_context(|| format!("Invalid YAML syntax in: {}", path.display()))?;

    println!("{}", "✓ Template is valid!".bright_green().bold());
    println!();
    println!(
        "{}",
        "Note: Full validation requires engine_templating operations module".yellow()
    );

    Ok(())
}

fn compile_template(path: &Path, output: Option<PathBuf>, all: bool, watch: bool) -> Result<()> {
    use engine_templating::TemplateCompiler;

    if watch {
        println!(
            "{}",
            "Watch mode is not yet implemented. Use --all to compile all templates once.".yellow()
        );
        println!();
        return Ok(());
    }

    let compiler = TemplateCompiler::new();

    // Compile directory recursively if --all is specified
    if all {
        if !path.exists() {
            anyhow::bail!("Directory not found: {}", path.display());
        }

        if !path.is_dir() {
            anyhow::bail!("Path is not a directory: {}", path.display());
        }

        println!(
            "{}",
            format!("Compiling all templates in: {}", path.display()).bright_blue().bold()
        );
        println!();

        let count = compiler
            .compile_directory(path)
            .with_context(|| format!("Failed to compile templates in: {}", path.display()))?;

        println!();
        println!(
            "{} {}",
            "✓".bright_green(),
            format!("Compiled {} template(s) successfully", count).bright_white()
        );

        return Ok(());
    }

    // Compile single template
    println!("{}", format!("Compiling template: {}", path.display()).bright_blue().bold());
    println!();

    if !path.exists() {
        anyhow::bail!("Template file not found: {}", path.display());
    }

    let output_path = output.unwrap_or_else(|| path.with_extension("bin"));

    // Compile the template
    let _compiled = compiler
        .compile(path, &output_path)
        .with_context(|| format!("Failed to compile template: {}", path.display()))?;

    // Get file sizes for reporting
    let yaml_size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    let bin_size = std::fs::metadata(&output_path).map(|m| m.len()).unwrap_or(0);

    let compression_ratio =
        if yaml_size > 0 { (bin_size as f64 / yaml_size as f64) * 100.0 } else { 100.0 };

    println!(
        "{} {}",
        "✓".bright_green(),
        format!("Compiled: {}", output_path.display()).bright_white()
    );

    // Load the compiled template to get entity count
    let template = compiler
        .load_compiled(&output_path)
        .with_context(|| format!("Failed to load compiled template: {}", output_path.display()))?;
    println!("   Entities: {}", template.entity_count());
    println!("   YAML size: {} bytes", yaml_size);
    println!("   Bincode size: {} bytes", bin_size);
    println!("   Compression: {:.1}%", compression_ratio);
    println!();
    println!("{}", "Template compiled successfully!".bright_green());
    println!();
    println!("{}", "Usage:".bright_blue().bold());
    println!("   The loader will automatically use the .bin file if it exists.");
    println!(
        "   Just load the template as usual: {}",
        format!("loader.load(&mut world, \"{}\")", path.display()).bright_cyan()
    );

    Ok(())
}

fn list_templates(base_path: Option<PathBuf>) -> Result<()> {
    let search_path = base_path.unwrap_or_else(|| PathBuf::from("assets/templates"));

    println!(
        "{}",
        format!("Listing templates in: {}", search_path.display()).bright_blue().bold()
    );
    println!();

    // TODO: Once engine_templating operations module is implemented, replace this with:
    // use engine_templating::operations::list_templates;
    //
    // let templates = list_templates(&search_path)
    //     .with_context(|| format!("Failed to list templates in: {}", search_path.display()))?;
    //
    // if templates.is_empty() {
    //     println!("{}", "No templates found.".dimmed());
    //     return Ok(());
    // }
    //
    // for template_info in templates {
    //     println!(
    //         "   {} {}",
    //         "•".bright_cyan(),
    //         template_info.path.display().to_string().bright_white()
    //     );
    //     if let Some(name) = template_info.metadata.name {
    //         println!("      {}", name.dimmed());
    //     }
    // }

    // Temporary stub implementation
    if !search_path.exists() {
        println!("{}", "No templates directory found.".dimmed());
        println!();
        println!("Create your first template with:");
        println!("   {}", "silm template add my_template --type level".bright_cyan());
        return Ok(());
    }

    let mut found_any = false;
    for entry in walkdir::WalkDir::new(&search_path)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.path().extension().and_then(|s| s.to_str()) == Some("yaml") {
            println!(
                "   {} {}",
                "•".bright_cyan(),
                entry.path().display().to_string().bright_white()
            );
            found_any = true;
        }
    }

    if !found_any {
        println!("{}", "No templates found.".dimmed());
        println!();
        println!("Create your first template with:");
        println!("   {}", "silm template add my_template --type level".bright_cyan());
    }

    Ok(())
}

fn show_template_tree(path: &Path) -> Result<()> {
    println!("{}", format!("Template hierarchy: {}", path.display()).bright_blue().bold());
    println!();

    // TODO: Once engine_templating operations module is implemented, replace this with:
    // use engine_templating::operations::show_template_tree;
    //
    // let tree = show_template_tree(path)
    //     .with_context(|| format!("Failed to load template tree: {}", path.display()))?;
    //
    // print_tree_node(&tree.root, 0);

    // Temporary stub implementation
    if !path.exists() {
        anyhow::bail!("Template file not found: {}", path.display());
    }

    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read template file: {}", path.display()))?;

    let parsed: serde_yaml::Value = serde_yaml::from_str(&content)
        .with_context(|| format!("Invalid YAML syntax in: {}", path.display()))?;

    // Basic tree printing
    if let Some(entities) = parsed.get("entities") {
        if let Some(mapping) = entities.as_mapping() {
            for (name, entity) in mapping {
                if let Some(name_str) = name.as_str() {
                    println!("   {} {}", "├─".bright_cyan(), name_str.bright_white());
                    print_entity_stub(entity, 1);
                }
            }
        }
    }

    println!();
    println!(
        "{}",
        "Note: Full tree display requires engine_templating operations module".yellow()
    );

    Ok(())
}

fn print_entity_stub(entity: &serde_yaml::Value, indent: usize) {
    let prefix = "   ".repeat(indent);

    if let Some(components) = entity.get("components") {
        if let Some(mapping) = components.as_mapping() {
            for (name, _) in mapping {
                if let Some(name_str) = name.as_str() {
                    if name_str != "tags" {
                        println!("   {}│  {} {}", prefix, "•".dimmed(), name_str.dimmed());
                    }
                }
            }
        }
    }

    if let Some(children) = entity.get("children") {
        if let Some(mapping) = children.as_mapping() {
            for (name, child) in mapping {
                if let Some(name_str) = name.as_str() {
                    println!("   {}├─ {}", prefix, name_str.bright_white());
                    print_entity_stub(child, indent + 1);
                }
            }
        }
    }
}

fn rename_template(path: &Path, new_name: &str) -> Result<()> {
    println!("{}", format!("Renaming template: {}", path.display()).bright_blue().bold());
    println!("   New name: {}", new_name.bright_cyan());
    println!();

    // TODO: Once engine_templating operations module is implemented, replace this with:
    // use engine_templating::operations::rename_template;
    //
    // let new_path = rename_template(path, new_name)
    //     .with_context(|| format!("Failed to rename template: {}", path.display()))?;
    //
    // println!(
    //     "{} {}",
    //     "✓".bright_green(),
    //     format!("Renamed to: {}", new_path.display()).bright_white()
    // );

    // Temporary stub implementation
    if !path.exists() {
        anyhow::bail!("Template file not found: {}", path.display());
    }

    let parent = path.parent().ok_or_else(|| {
        anyhow::anyhow!("Cannot determine parent directory for: {}", path.display())
    })?;

    let new_path = parent.join(format!("{}.yaml", new_name));

    if new_path.exists() {
        anyhow::bail!(
            "A template with the name '{}' already exists at: {}",
            new_name,
            new_path.display()
        );
    }

    std::fs::rename(path, &new_path)
        .with_context(|| format!("Failed to rename file: {}", path.display()))?;

    println!(
        "{} {}",
        "✓".bright_green(),
        format!("Renamed to: {}", new_path.display()).bright_white()
    );

    Ok(())
}

fn delete_template(path: &Path, skip_confirm: bool) -> Result<()> {
    if !path.exists() {
        anyhow::bail!("Template file not found: {}", path.display());
    }

    println!("{}", format!("Delete template: {}", path.display()).bright_red().bold());
    println!();

    if !skip_confirm {
        println!("{}", "This action cannot be undone!".yellow());
        print!("Are you sure? [y/N]: ");
        use std::io::Write;
        std::io::stdout().flush()?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        let input = input.trim().to_lowercase();
        if input != "y" && input != "yes" {
            println!("{}", "Cancelled.".dimmed());
            return Ok(());
        }
    }

    // TODO: Once engine_templating operations module is implemented, replace this with:
    // use engine_templating::operations::delete_template;
    //
    // delete_template(path)
    //     .with_context(|| format!("Failed to delete template: {}", path.display()))?;

    // Temporary stub implementation
    std::fs::remove_file(path)
        .with_context(|| format!("Failed to delete file: {}", path.display()))?;

    println!("{} {}", "✓".bright_green(), "Template deleted.".bright_white());

    Ok(())
}
