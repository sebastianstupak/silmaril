//! Template validation infrastructure.
//!
//! Validates template YAML files for:
//! - YAML syntax correctness
//! - Component type validity
//! - Template reference existence
//! - Circular dependency detection (direct, indirect, deep)
//!
//! # Examples
//!
//! ```rust,no_run
//! use engine_templating::validator::{TemplateValidator, ValidationReport};
//! use std::path::Path;
//!
//! let validator = TemplateValidator::new();
//! let report = validator.validate(Path::new("templates/player.yaml"))?;
//!
//! if report.is_valid {
//!     println!("Template is valid!");
//!     println!("Entity count: {}", report.entity_count);
//!     println!("Template references: {:?}", report.template_references);
//! } else {
//!     for error in &report.errors {
//!         eprintln!("Error: {}", error);
//!     }
//! }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use crate::error::{TemplateError, TemplateResult};
use crate::template::{EntityDefinition, EntitySource, Template};
use rustc_hash::FxHashSet;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

/// Template validator for checking YAML syntax, component types, and circular dependencies.
///
/// # Examples
///
/// ```rust
/// use engine_templating::validator::TemplateValidator;
/// use std::path::Path;
///
/// let validator = TemplateValidator::new();
/// ```
#[derive(Debug)]
pub struct TemplateValidator {
    /// Set of known component types (e.g., "Transform", "Health", "Camera")
    known_components: FxHashSet<String>,
}

impl TemplateValidator {
    /// Creates a new template validator with default known components.
    ///
    /// The default set includes common engine components like Transform, Health, etc.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use engine_templating::validator::TemplateValidator;
    ///
    /// let validator = TemplateValidator::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        let mut known_components = FxHashSet::default();

        // Core components (from engine-core)
        known_components.insert("Transform".to_string());
        known_components.insert("Health".to_string());
        known_components.insert("Velocity".to_string());

        // Rendering components (from engine-renderer)
        known_components.insert("Camera".to_string());
        known_components.insert("MeshRenderer".to_string());

        // Physics components (from engine-physics)
        known_components.insert("Collider".to_string());
        known_components.insert("RigidBody".to_string());

        // Character components
        known_components.insert("CharacterController".to_string());

        // Networking components
        known_components.insert("NetworkIdentity".to_string());

        Self { known_components }
    }

    /// Registers a custom component type as valid.
    ///
    /// Use this to add game-specific component types beyond the default set.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use engine_templating::validator::TemplateValidator;
    ///
    /// let mut validator = TemplateValidator::new();
    /// validator.register_component("CustomWeapon".to_string());
    /// ```
    pub fn register_component(&mut self, component_name: String) {
        self.known_components.insert(component_name);
    }

    /// Validates a template file at the given path.
    ///
    /// Checks for:
    /// - YAML syntax errors
    /// - Unknown component types
    /// - Missing template references
    /// - Circular dependencies (direct, indirect, deep)
    ///
    /// # Arguments
    ///
    /// * `template_path` - Path to the template YAML file to validate
    ///
    /// # Returns
    ///
    /// A `ValidationReport` containing validation results, errors, and warnings.
    ///
    /// # Errors
    ///
    /// Returns `TemplateError::Io` if the file cannot be read.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use engine_templating::validator::TemplateValidator;
    /// use std::path::Path;
    ///
    /// let validator = TemplateValidator::new();
    /// let report = validator.validate(Path::new("templates/player.yaml"))?;
    ///
    /// assert!(report.is_valid);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn validate(&self, template_path: &Path) -> TemplateResult<ValidationReport> {
        info!(
            path = %template_path.display(),
            "Validating template"
        );

        let mut report = ValidationReport::new();

        // Step 1: Read file
        let yaml_content = fs::read_to_string(template_path).map_err(|e| {
            debug!(
                path = %template_path.display(),
                error = ?e,
                "Failed to read template file"
            );
            TemplateError::io(template_path.display().to_string(), e.to_string())
        })?;

        // Step 2: Parse YAML
        let template: Template = match serde_yaml::from_str(&yaml_content) {
            Ok(t) => t,
            Err(e) => {
                let error_msg = format!("Invalid YAML syntax: {}", e);
                report.errors.push(error_msg.clone());
                report.is_valid = false;
                warn!(
                    path = %template_path.display(),
                    error = %e,
                    "YAML parsing failed"
                );
                return Ok(report);
            }
        };

        // Step 3: Count entities (including children)
        report.entity_count = self.count_entities(&template);

        // Step 4: Validate components and collect template references
        self.validate_components_and_references(
            &template,
            template_path,
            &mut report.errors,
            &mut report.warnings,
            &mut report.template_references,
        );

        // Step 5: Check for circular dependencies
        if !report.template_references.is_empty() {
            if let Err(e) = self.check_circular_dependencies(template_path, &template) {
                report.errors.push(e.to_string());
            }
        }

        // Step 6: Determine overall validity
        report.is_valid = report.errors.is_empty();

        if report.is_valid {
            info!(
                path = %template_path.display(),
                entity_count = report.entity_count,
                warnings = report.warnings.len(),
                "Template validation passed"
            );
        } else {
            warn!(
                path = %template_path.display(),
                error_count = report.errors.len(),
                "Template validation failed"
            );
        }

        Ok(report)
    }

    /// Counts total number of entities (including nested children).
    fn count_entities(&self, template: &Template) -> usize {
        let mut count = 0;
        for entity in template.entities.values() {
            count += 1;
            count += self.count_entity_children(entity);
        }
        count
    }

    /// Recursively counts child entities.
    fn count_entity_children(&self, entity: &EntityDefinition) -> usize {
        let mut count = 0;
        for child in entity.children.values() {
            count += 1;
            count += self.count_entity_children(child);
        }
        count
    }

    /// Validates component types and collects template references.
    fn validate_components_and_references(
        &self,
        template: &Template,
        template_path: &Path,
        errors: &mut Vec<String>,
        warnings: &mut Vec<String>,
        template_references: &mut Vec<String>,
    ) {
        for (entity_name, entity) in &template.entities {
            self.validate_entity_definition(
                entity_name,
                entity,
                template_path,
                errors,
                warnings,
                template_references,
            );
        }
    }

    /// Validates a single entity definition and its children.
    fn validate_entity_definition(
        &self,
        entity_name: &str,
        entity: &EntityDefinition,
        template_path: &Path,
        errors: &mut Vec<String>,
        warnings: &mut Vec<String>,
        template_references: &mut Vec<String>,
    ) {
        match &entity.source {
            EntitySource::Inline { components, tags } => {
                // Validate component types
                for component_name in components.keys() {
                    if !self.known_components.contains(component_name) {
                        errors.push(format!(
                            "Entity '{}': Unknown component type '{}'",
                            entity_name, component_name
                        ));
                    }
                }

                // Warn if entity has no components
                if components.is_empty() && tags.is_empty() && entity.children.is_empty() {
                    warnings.push(format!(
                        "Entity '{}' has no components, tags, or children (unused entity)",
                        entity_name
                    ));
                }
            }
            EntitySource::Reference { template: ref_path } => {
                // Collect template reference
                if !template_references.contains(ref_path) {
                    template_references.push(ref_path.clone());
                }

                // Validate template reference exists
                let base_dir = template_path.parent().unwrap_or_else(|| Path::new("."));
                let resolved_path = base_dir.join(ref_path);

                if !resolved_path.exists() {
                    errors.push(format!(
                        "Entity '{}': Template reference '{}' does not exist (resolved to '{}')",
                        entity_name,
                        ref_path,
                        resolved_path.display()
                    ));
                }
            }
        }

        // Validate override components
        for component_name in entity.overrides.keys() {
            if !self.known_components.contains(component_name) {
                errors.push(format!(
                    "Entity '{}': Unknown component type in overrides '{}'",
                    entity_name, component_name
                ));
            }
        }

        // Recursively validate children
        for (child_name, child) in &entity.children {
            self.validate_entity_definition(
                &format!("{}.{}", entity_name, child_name),
                child,
                template_path,
                errors,
                warnings,
                template_references,
            );
        }
    }

    /// Checks for circular dependencies in template references.
    ///
    /// Detects:
    /// - Direct cycles: A → A
    /// - Indirect cycles: A → B → A
    /// - Deep cycles: A → B → C → A
    fn check_circular_dependencies(
        &self,
        template_path: &Path,
        template: &Template,
    ) -> TemplateResult<()> {
        let mut visited = FxHashSet::default();
        let mut stack = Vec::new();

        self.detect_cycles_recursive(template_path, template, &mut visited, &mut stack)
    }

    /// Recursively detects cycles in template reference graph.
    fn detect_cycles_recursive(
        &self,
        current_path: &Path,
        template: &Template,
        visited: &mut FxHashSet<PathBuf>,
        stack: &mut Vec<PathBuf>,
    ) -> TemplateResult<()> {
        let canonical_path =
            current_path.canonicalize().unwrap_or_else(|_| current_path.to_path_buf());

        // Check if we're already in the current traversal stack (cycle detected)
        if stack.contains(&canonical_path) {
            // Build cycle path string
            let cycle_start = stack.iter().position(|p| p == &canonical_path).unwrap_or(0);
            let cycle_path: Vec<String> = stack[cycle_start..]
                .iter()
                .map(|p| p.display().to_string())
                .chain(std::iter::once(canonical_path.display().to_string()))
                .collect();

            let cycle_str = cycle_path.join(" → ");
            debug!(cycle = %cycle_str, "Circular dependency detected");
            return Err(TemplateError::circularreference(cycle_str));
        }

        // Already fully processed this template
        if visited.contains(&canonical_path) {
            return Ok(());
        }

        // Add to current traversal stack
        stack.push(canonical_path.clone());

        // Collect all template references from this template
        let mut references = Vec::new();
        for entity in template.entities.values() {
            self.collect_template_references(entity, &mut references);
        }

        // Recursively check each referenced template
        for ref_path in references {
            let base_dir = current_path.parent().unwrap_or_else(|| Path::new("."));
            let resolved_path = base_dir.join(&ref_path);

            if !resolved_path.exists() {
                // Skip non-existent references (already reported as error)
                continue;
            }

            // Load and validate referenced template
            let yaml_content = fs::read_to_string(&resolved_path).map_err(|e| {
                TemplateError::io(resolved_path.display().to_string(), e.to_string())
            })?;
            let referenced_template: Template = serde_yaml::from_str(&yaml_content)
                .map_err(|e| TemplateError::invalidyaml(e.to_string()))?;

            self.detect_cycles_recursive(&resolved_path, &referenced_template, visited, stack)?;
        }

        // Remove from stack and mark as visited
        stack.pop();
        visited.insert(canonical_path);

        Ok(())
    }

    /// Recursively collects all template references from an entity definition.
    fn collect_template_references(&self, entity: &EntityDefinition, references: &mut Vec<String>) {
        if let EntitySource::Reference { template } = &entity.source {
            references.push(template.clone());
        }

        for child in entity.children.values() {
            self.collect_template_references(child, references);
        }
    }
}

impl Default for TemplateValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Validation report containing results, errors, warnings, and statistics.
///
/// # Examples
///
/// ```rust
/// use engine_templating::validator::ValidationReport;
///
/// let mut report = ValidationReport::new();
/// report.errors.push("Missing component".to_string());
/// report.is_valid = report.errors.is_empty();
///
/// assert!(!report.is_valid);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationReport {
    /// True if the template is valid (no errors)
    pub is_valid: bool,

    /// List of validation errors (prevent template from loading)
    pub errors: Vec<String>,

    /// List of validation warnings (template can load, but might have issues)
    pub warnings: Vec<String>,

    /// Total number of entities in the template (including children)
    pub entity_count: usize,

    /// List of template references found in this template
    pub template_references: Vec<String>,
}

impl ValidationReport {
    /// Creates a new empty validation report.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use engine_templating::validator::ValidationReport;
    ///
    /// let report = ValidationReport::new();
    /// assert!(report.is_valid);
    /// assert_eq!(report.errors.len(), 0);
    /// assert_eq!(report.warnings.len(), 0);
    /// assert_eq!(report.entity_count, 0);
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
            entity_count: 0,
            template_references: Vec::new(),
        }
    }
}

impl Default for ValidationReport {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::template::TemplateMetadata;
    use rustc_hash::FxHashMap;

    #[test]
    fn test_validator_creation() {
        let validator = TemplateValidator::new();
        assert!(validator.known_components.contains("Transform"));
        assert!(validator.known_components.contains("Health"));
        assert!(validator.known_components.contains("Camera"));
    }

    #[test]
    fn test_register_custom_component() {
        let mut validator = TemplateValidator::new();
        validator.register_component("CustomWeapon".to_string());
        assert!(validator.known_components.contains("CustomWeapon"));
    }

    #[test]
    fn test_validation_report_new() {
        let report = ValidationReport::new();
        assert!(report.is_valid);
        assert_eq!(report.errors.len(), 0);
        assert_eq!(report.warnings.len(), 0);
        assert_eq!(report.entity_count, 0);
        assert_eq!(report.template_references.len(), 0);
    }

    #[test]
    fn test_count_entities_single() {
        let validator = TemplateValidator::new();
        let mut template = Template::new(TemplateMetadata::default());

        let entity = EntityDefinition::new_inline(FxHashMap::default(), vec![]);
        template.add_entity("Root".to_string(), entity);

        assert_eq!(validator.count_entities(&template), 1);
    }

    #[test]
    fn test_count_entities_with_children() {
        let validator = TemplateValidator::new();
        let mut template = Template::new(TemplateMetadata::default());

        let mut parent = EntityDefinition::new_inline(FxHashMap::default(), vec![]);
        let child = EntityDefinition::new_inline(FxHashMap::default(), vec![]);
        parent.add_child("Camera".to_string(), child);

        template.add_entity("Root".to_string(), parent);

        assert_eq!(validator.count_entities(&template), 2);
    }
}
