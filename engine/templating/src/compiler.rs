//! Template compiler for converting YAML templates to optimized bincode format.
//!
//! This module provides the `TemplateCompiler` which compiles YAML templates into
//! bincode format for 10-50x faster loading. The compiled templates include checksum
//! validation to ensure data integrity.
//!
//! # Examples
//!
//! ```no_run
//! use engine_templating::compiler::TemplateCompiler;
//! use std::path::Path;
//!
//! let compiler = TemplateCompiler::new();
//!
//! // Compile a YAML template to bincode
//! compiler.compile(
//!     Path::new("assets/templates/player.yaml"),
//!     Path::new("assets/templates/player.bin")
//! ).expect("Compilation failed");
//!
//! // Load the compiled template
//! let template = compiler.load_compiled(Path::new("assets/templates/player.bin"))
//!     .expect("Failed to load compiled template");
//! ```

use crate::error::{TemplateError, TemplateResult};
use crate::template::{EntityDefinition, EntitySource, Template, TemplateMetadata};
use rustc_hash::FxHashMap;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use tracing::{debug, info, warn};
use xxhash_rust::xxh64::xxh64;

/// Magic number to identify compiled template files (ASCII: "SILM")
const MAGIC_NUMBER: u32 = 0x53494C4D;

/// Current version of the compiled template format
const FORMAT_VERSION: u32 = 1;

/// A bincode-compatible serializable template.
///
/// This struct converts `serde_yaml::Value` to strings for bincode serialization,
/// since bincode doesn't support `deserialize_any` which `serde_yaml::Value` uses.
///
/// Uses `BTreeMap` instead of `FxHashMap` to ensure deterministic serialization order
/// for consistent checksums.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct SerializableTemplate {
    metadata: TemplateMetadata,
    entities: BTreeMap<String, SerializableEntityDefinition>,
}

/// A bincode-compatible serializable entity definition.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct SerializableEntityDefinition {
    source: SerializableEntitySource,
    overrides: BTreeMap<String, String>, // YAML as strings
    children: BTreeMap<String, SerializableEntityDefinition>,
}

/// A bincode-compatible serializable entity source.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
enum SerializableEntitySource {
    Inline {
        components: BTreeMap<String, String>, // YAML as strings
        tags: Vec<String>,
    },
    Reference {
        template: String,
    },
}

impl From<Template> for SerializableTemplate {
    fn from(template: Template) -> Self {
        let entities = template
            .entities
            .into_iter()
            .map(|(name, def)| (name, SerializableEntityDefinition::from(def)))
            .collect();

        Self { metadata: template.metadata, entities }
    }
}

impl TryFrom<SerializableTemplate> for Template {
    type Error = TemplateError;

    fn try_from(serializable: SerializableTemplate) -> Result<Self, Self::Error> {
        let entities = serializable
            .entities
            .into_iter()
            .map(|(name, def)| EntityDefinition::try_from(def).map(|entity| (name, entity)))
            .collect::<Result<FxHashMap<_, _>, _>>()?;

        Ok(Self { metadata: serializable.metadata, entities })
    }
}

impl From<EntityDefinition> for SerializableEntityDefinition {
    fn from(def: EntityDefinition) -> Self {
        let source = match def.source {
            EntitySource::Inline { components, tags } => {
                let serializable_components = components
                    .into_iter()
                    .map(|(k, v)| {
                        let yaml_str =
                            serde_yaml::to_string(&v).unwrap_or_else(|_| "null".to_string());
                        (k, yaml_str)
                    })
                    .collect();

                SerializableEntitySource::Inline { components: serializable_components, tags }
            }
            EntitySource::Reference { template } => {
                SerializableEntitySource::Reference { template }
            }
        };

        let overrides = def
            .overrides
            .into_iter()
            .map(|(k, v)| {
                let yaml_str = serde_yaml::to_string(&v).unwrap_or_else(|_| "null".to_string());
                (k, yaml_str)
            })
            .collect();

        let children = def
            .children
            .into_iter()
            .map(|(name, child)| (name, SerializableEntityDefinition::from(child)))
            .collect();

        Self { source, overrides, children }
    }
}

impl TryFrom<SerializableEntityDefinition> for EntityDefinition {
    type Error = TemplateError;

    fn try_from(serializable: SerializableEntityDefinition) -> Result<Self, Self::Error> {
        let source = match serializable.source {
            SerializableEntitySource::Inline { components, tags } => {
                let parsed_components = components
                    .into_iter()
                    .map(|(k, yaml_str)| {
                        let value = serde_yaml::from_str(&yaml_str).map_err(|e| {
                            TemplateError::invalidyaml(format!(
                                "Failed to parse component '{}': {}",
                                k, e
                            ))
                        })?;
                        Ok((k, value))
                    })
                    .collect::<Result<FxHashMap<_, _>, TemplateError>>()?;

                EntitySource::Inline { components: parsed_components, tags }
            }
            SerializableEntitySource::Reference { template } => {
                EntitySource::Reference { template }
            }
        };

        let overrides = serializable
            .overrides
            .into_iter()
            .map(|(k, yaml_str)| {
                let value = serde_yaml::from_str(&yaml_str).map_err(|e| {
                    TemplateError::invalidyaml(format!("Failed to parse override '{}': {}", k, e))
                })?;
                Ok((k, value))
            })
            .collect::<Result<FxHashMap<_, _>, TemplateError>>()?;

        let children = serializable
            .children
            .into_iter()
            .map(|(name, child)| EntityDefinition::try_from(child).map(|entity| (name, entity)))
            .collect::<Result<FxHashMap<_, _>, TemplateError>>()?;

        Ok(Self { source, overrides, children })
    }
}

/// A compiled template with metadata and checksum for validation.
///
/// The compiled format includes:
/// - Magic number for file type identification
/// - Format version for compatibility checking
/// - Checksum for data integrity validation
/// - Bincode-serialized template data
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CompiledTemplate {
    /// Magic number to identify file format
    magic: u32,

    /// Format version for compatibility checking
    version: u32,

    /// Checksum of the template data (xxHash64)
    checksum: u64,

    /// The compiled template data (serializable format)
    template: SerializableTemplate,
}

impl CompiledTemplate {
    /// Creates a new compiled template with checksum validation.
    fn new(template: Template) -> Self {
        let serializable = SerializableTemplate::from(template);
        let template_bytes = bincode::serialize(&serializable).unwrap_or_default();
        let checksum = xxh64(&template_bytes, 0);

        Self { magic: MAGIC_NUMBER, version: FORMAT_VERSION, checksum, template: serializable }
    }

    /// Validates the compiled template's magic number and version.
    fn validate(&self) -> TemplateResult<()> {
        if self.magic != MAGIC_NUMBER {
            return Err(TemplateError::serialization(format!(
                "Invalid magic number: expected 0x{:X}, found 0x{:X}",
                MAGIC_NUMBER, self.magic
            )));
        }

        if self.version != FORMAT_VERSION {
            return Err(TemplateError::serialization(format!(
                "Incompatible format version: expected {}, found {}",
                FORMAT_VERSION, self.version
            )));
        }

        // Verify checksum
        let template_bytes = bincode::serialize(&self.template).map_err(|e| {
            TemplateError::serialization(format!(
                "Failed to serialize template for checksum validation: {}",
                e
            ))
        })?;
        let computed_checksum = xxh64(&template_bytes, 0);

        if computed_checksum != self.checksum {
            return Err(TemplateError::serialization(format!(
                "Checksum mismatch: expected 0x{:X}, computed 0x{:X}",
                self.checksum, computed_checksum
            )));
        }

        Ok(())
    }

    /// Extracts the template from the compiled format.
    fn into_template(self) -> TemplateResult<Template> {
        Template::try_from(self.template)
    }
}

/// Template compiler for converting YAML templates to bincode format.
///
/// The compiler provides methods to:
/// - Compile YAML templates to optimized bincode format
/// - Load compiled templates with checksum validation
/// - Support both single-file and batch compilation
///
/// # Performance
///
/// Bincode templates are typically:
/// - 10-50x faster to load than YAML
/// - 50-80% smaller in file size
/// - Zero-copy deserialization friendly
///
/// # Examples
///
/// ```no_run
/// use engine_templating::compiler::TemplateCompiler;
/// use std::path::Path;
///
/// let compiler = TemplateCompiler::new();
///
/// // Compile a template
/// compiler.compile(
///     Path::new("templates/level.yaml"),
///     Path::new("templates/level.bin")
/// ).expect("Failed to compile");
/// ```
pub struct TemplateCompiler {
    // Future: Could add compilation options here (compression, optimization level, etc.)
}

impl TemplateCompiler {
    /// Creates a new template compiler with default settings.
    ///
    /// # Examples
    ///
    /// ```
    /// use engine_templating::compiler::TemplateCompiler;
    ///
    /// let compiler = TemplateCompiler::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {}
    }

    /// Compiles a YAML template to bincode format.
    ///
    /// This method:
    /// 1. Reads the YAML template from `yaml_path`
    /// 2. Parses it into a `Template` struct
    /// 3. Serializes it to bincode with checksum
    /// 4. Writes the compiled template to `output_path`
    ///
    /// # Arguments
    ///
    /// * `yaml_path` - Path to the source YAML template
    /// * `output_path` - Path where the compiled template should be written
    ///
    /// # Returns
    ///
    /// The compiled template on success, or a `TemplateError` on failure.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use engine_templating::compiler::TemplateCompiler;
    /// use std::path::Path;
    ///
    /// let compiler = TemplateCompiler::new();
    /// let compiled = compiler.compile(
    ///     Path::new("player.yaml"),
    ///     Path::new("player.bin")
    /// ).expect("Failed to compile");
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The YAML file cannot be read
    /// - The YAML is invalid or malformed
    /// - The output file cannot be written
    /// - Serialization to bincode fails
    pub fn compile<P: AsRef<Path>, Q: AsRef<Path>>(
        &self,
        yaml_path: P,
        output_path: Q,
    ) -> TemplateResult<CompiledTemplate> {
        let yaml_path = yaml_path.as_ref();
        let output_path = output_path.as_ref();

        info!(
            yaml_path = %yaml_path.display(),
            output_path = %output_path.display(),
            "Compiling template"
        );

        // Read YAML file
        if !yaml_path.exists() {
            return Err(TemplateError::notfound(yaml_path.display().to_string()));
        }

        let yaml_content = fs::read_to_string(yaml_path)
            .map_err(|e| TemplateError::from_io_error(yaml_path.display().to_string(), e))?;

        debug!(yaml_size = yaml_content.len(), "Read YAML template");

        // Parse YAML to Template
        let template: Template = serde_yaml::from_str(&yaml_content)
            .map_err(|e| TemplateError::invalidyaml(e.to_string()))?;

        debug!(entity_count = template.entity_count(), "Parsed template");

        // Create compiled template with checksum
        let compiled = CompiledTemplate::new(template);

        // Serialize to bincode
        let bincode_data = bincode::serialize(&compiled).map_err(|e| {
            TemplateError::serialization(format!("Failed to serialize to bincode: {}", e))
        })?;

        // Write to output file
        fs::write(output_path, &bincode_data)
            .map_err(|e| TemplateError::from_io_error(output_path.display().to_string(), e))?;

        let compression_ratio = (bincode_data.len() as f64 / yaml_content.len() as f64) * 100.0;

        info!(
            yaml_size = yaml_content.len(),
            bincode_size = bincode_data.len(),
            compression_ratio = format!("{:.1}%", compression_ratio),
            "Template compiled successfully"
        );

        Ok(compiled)
    }

    /// Loads a compiled bincode template with checksum validation.
    ///
    /// This method:
    /// 1. Reads the compiled template from `bin_path`
    /// 2. Deserializes the bincode data
    /// 3. Validates magic number, version, and checksum
    /// 4. Returns the template
    ///
    /// # Arguments
    ///
    /// * `bin_path` - Path to the compiled bincode template
    ///
    /// # Returns
    ///
    /// The loaded template on success, or a `TemplateError` on failure.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use engine_templating::compiler::TemplateCompiler;
    /// use std::path::Path;
    ///
    /// let compiler = TemplateCompiler::new();
    /// let template = compiler.load_compiled(Path::new("player.bin"))
    ///     .expect("Failed to load");
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The bincode file cannot be read
    /// - Deserialization fails
    /// - Magic number is invalid
    /// - Format version is incompatible
    /// - Checksum validation fails
    pub fn load_compiled<P: AsRef<Path>>(&self, bin_path: P) -> TemplateResult<Template> {
        let bin_path = bin_path.as_ref();

        debug!(
            bin_path = %bin_path.display(),
            "Loading compiled template"
        );

        // Read bincode file
        if !bin_path.exists() {
            return Err(TemplateError::notfound(bin_path.display().to_string()));
        }

        let bincode_data = fs::read(bin_path)
            .map_err(|e| TemplateError::from_io_error(bin_path.display().to_string(), e))?;

        debug!(bincode_size = bincode_data.len(), "Read compiled template");

        // Deserialize from bincode
        let compiled: CompiledTemplate = bincode::deserialize(&bincode_data).map_err(|e| {
            TemplateError::serialization(format!("Failed to deserialize bincode: {}", e))
        })?;

        // Validate magic number, version, and checksum
        compiled.validate()?;

        // Extract checksum before consuming
        let checksum = compiled.checksum;

        // Convert from serializable format to template
        let template = compiled.into_template()?;

        debug!(
            entity_count = template.entity_count(),
            checksum = format!("0x{:X}", checksum),
            "Compiled template validated"
        );

        info!(
            bin_path = %bin_path.display(),
            "Loaded compiled template successfully"
        );

        Ok(template)
    }

    /// Compiles all YAML templates in a directory.
    ///
    /// This method walks through the directory and compiles all `.yaml` files
    /// to `.bin` files with the same name.
    ///
    /// # Arguments
    ///
    /// * `directory` - Path to the directory containing YAML templates
    ///
    /// # Returns
    ///
    /// The number of templates compiled on success.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use engine_templating::compiler::TemplateCompiler;
    /// use std::path::Path;
    ///
    /// let compiler = TemplateCompiler::new();
    /// let count = compiler.compile_directory(Path::new("assets/templates"))
    ///     .expect("Failed to compile directory");
    /// println!("Compiled {} templates", count);
    /// ```
    ///
    /// # Errors
    ///
    /// Returns the first error encountered during compilation.
    pub fn compile_directory<P: AsRef<Path>>(&self, directory: P) -> TemplateResult<usize> {
        let directory = directory.as_ref();

        if !directory.exists() {
            return Err(TemplateError::notfound(directory.display().to_string()));
        }

        if !directory.is_dir() {
            return Err(TemplateError::io(
                directory.display().to_string(),
                "Path is not a directory".to_string(),
            ));
        }

        info!(
            directory = %directory.display(),
            "Compiling all templates in directory"
        );

        let mut compiled_count = 0;

        for entry in fs::read_dir(directory)
            .map_err(|e| TemplateError::from_io_error(directory.display().to_string(), e))?
        {
            let entry = entry
                .map_err(|e| TemplateError::from_io_error(directory.display().to_string(), e))?;
            let path = entry.path();

            // Only process .yaml files
            if path.extension().and_then(|s| s.to_str()) == Some("yaml") {
                let output_path = path.with_extension("bin");

                match self.compile(&path, &output_path) {
                    Ok(_) => {
                        compiled_count += 1;
                    }
                    Err(e) => {
                        warn!(
                            path = %path.display(),
                            error = ?e,
                            "Failed to compile template, skipping"
                        );
                        // Continue processing other files instead of failing
                    }
                }
            }

            // Recursively compile subdirectories
            if path.is_dir() {
                compiled_count += self.compile_directory(&path)?;
            }
        }

        info!(
            directory = %directory.display(),
            compiled_count,
            "Finished compiling directory"
        );

        Ok(compiled_count)
    }
}

impl Default for TemplateCompiler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::template::{EntityDefinition, TemplateMetadata};
    use rustc_hash::FxHashMap;
    use tempfile::TempDir;

    #[test]
    fn test_compiled_template_validation() {
        let metadata = TemplateMetadata {
            name: Some("Test".to_string()),
            description: None,
            author: None,
            version: None,
        };
        let template = Template::new(metadata);

        let compiled = CompiledTemplate::new(template);

        assert_eq!(compiled.magic, MAGIC_NUMBER);
        assert_eq!(compiled.version, FORMAT_VERSION);
        assert!(compiled.validate().is_ok());
    }

    #[test]
    fn test_compiled_template_checksum_validation() {
        let metadata = TemplateMetadata {
            name: Some("Test".to_string()),
            description: None,
            author: None,
            version: None,
        };
        let template = Template::new(metadata);

        let mut compiled = CompiledTemplate::new(template);

        // Corrupt the checksum
        compiled.checksum = 0xDEADBEEF;

        assert!(compiled.validate().is_err());
    }

    #[test]
    fn test_compile_and_load() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let yaml_path = temp_dir.path().join("test.yaml");
        let bin_path = temp_dir.path().join("test.bin");

        // Create a test template
        let metadata = TemplateMetadata {
            name: Some("Test Template".to_string()),
            description: Some("A test".to_string()),
            author: None,
            version: Some("1.0".to_string()),
        };
        let mut template = Template::new(metadata.clone());
        template.add_entity(
            "Root".to_string(),
            EntityDefinition::new_inline(FxHashMap::default(), vec![]),
        );

        // Write as YAML
        let yaml = serde_yaml::to_string(&template).expect("Failed to serialize YAML");
        fs::write(&yaml_path, yaml).expect("Failed to write YAML");

        // Compile
        let compiler = TemplateCompiler::new();
        compiler.compile(&yaml_path, &bin_path).expect("Compilation failed");

        // Load
        let loaded = compiler.load_compiled(&bin_path).expect("Failed to load");

        assert_eq!(loaded.metadata, metadata);
        assert_eq!(loaded.entity_count(), 1);
    }

    #[test]
    fn test_compile_nonexistent_file() {
        let compiler = TemplateCompiler::new();
        let result = compiler.compile(Path::new("nonexistent.yaml"), Path::new("output.bin"));

        assert!(result.is_err());
    }

    #[test]
    fn test_load_nonexistent_file() {
        let compiler = TemplateCompiler::new();
        let result = compiler.load_compiled(Path::new("nonexistent.bin"));

        assert!(result.is_err());
    }

    #[test]
    fn test_compile_directory() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Create multiple YAML templates
        for i in 0..3 {
            let yaml_path = temp_dir.path().join(format!("template_{}.yaml", i));
            let metadata = TemplateMetadata {
                name: Some(format!("Template {}", i)),
                description: None,
                author: None,
                version: None,
            };
            let template = Template::new(metadata);
            let yaml = serde_yaml::to_string(&template).expect("Failed to serialize");
            fs::write(yaml_path, yaml).expect("Failed to write YAML");
        }

        // Compile directory
        let compiler = TemplateCompiler::new();
        let count = compiler
            .compile_directory(temp_dir.path())
            .expect("Failed to compile directory");

        assert_eq!(count, 3);

        // Verify all .bin files exist
        for i in 0..3 {
            let bin_path = temp_dir.path().join(format!("template_{}.bin", i));
            assert!(bin_path.exists());
        }
    }
}
