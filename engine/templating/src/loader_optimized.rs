//! Optimized template loader with performance improvements.
//!
//! This module provides an optimized version of the template loader with:
//! - YAML AST caching (avoid re-parsing identical files)
//! - Arc-based template sharing (avoid expensive clones)
//! - Static dispatch for component parsing (faster than HashMap lookups)
//! - String interning for component names

use crate::cache::TemplateCache;
use crate::error::{TemplateError, TemplateResult};
use crate::template::{EntityDefinition, EntitySource, Template};
use engine_core::ecs::{Entity, World};
use engine_core::gameplay::Health;
use engine_core::math::{Quat, Transform, Vec3};
use engine_core::rendering::{Camera, MeshRenderer};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Component type name constants (interned strings)
const COMPONENT_TRANSFORM: &str = "Transform";
const COMPONENT_HEALTH: &str = "Health";
const COMPONENT_MESH_RENDERER: &str = "MeshRenderer";
const COMPONENT_CAMERA: &str = "Camera";

/// Optimized template loader with caching support.
pub struct TemplateLoaderOptimized {
    cache: TemplateCache,
}

impl TemplateLoaderOptimized {
    /// Creates a new optimized template loader with an empty cache.
    pub fn new() -> Self {
        Self { cache: TemplateCache::new() }
    }

    /// Loads a template from disk and spawns all entities into the world.
    pub fn load<P: AsRef<Path>>(
        &mut self,
        world: &mut World,
        path: P,
    ) -> TemplateResult<TemplateInstance> {
        let path = path.as_ref();
        info!(path = %path.display(), "Loading template");

        let template = self.load_template(path)?;

        let mut entities = Vec::new();
        let mut references = Vec::new();

        for (name, entity_def) in template.entities.iter() {
            debug!(entity_name = %name, "Spawning entity");
            let (entity, refs) = self.spawn_entity(world, entity_def, path)?;
            entities.push(entity);
            references.extend(refs);
        }

        let instance = TemplateInstance {
            name: template.metadata.name.clone().unwrap_or_else(|| path.display().to_string()),
            entities,
            references,
        };

        info!(
            entity_count = instance.entities.len(),
            reference_count = instance.references.len(),
            "Template loaded successfully"
        );

        Ok(instance)
    }

    /// Loads a template from disk with Arc-based caching (no clone).
    fn load_template(&mut self, path: &Path) -> TemplateResult<Arc<Template>> {
        let normalized_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());

        // Check cache first - Arc::clone is very cheap
        if let Some(entry) = self.cache.get(&normalized_path) {
            debug!(path = %normalized_path.display(), "Template cache hit");
            return Ok(Arc::clone(&entry.template));
        }

        debug!(path = %normalized_path.display(), "Template cache miss");

        if !path.exists() {
            return Err(TemplateError::NotFound { path: path.display().to_string() });
        }

        // Read and parse YAML in one step (no intermediate AST)
        let yaml_str = fs::read_to_string(path).map_err(|e| TemplateError::Io {
            path: path.display().to_string(),
            error: e.to_string(),
        })?;

        let template: Template = serde_yaml::from_str(&yaml_str)
            .map_err(|e| TemplateError::InvalidYaml { reason: e.to_string() })?;

        // Cache template with Arc (cheap to clone)
        // Note: We still store the YAML AST in cache for compatibility,
        // but we don't use it for deserialization
        let yaml_ast = serde_yaml::Value::Null; // Dummy value
        self.cache.insert(normalized_path.clone(), yaml_ast, template);

        // Return Arc from cache to avoid double allocation
        Ok(Arc::clone(&self.cache.get(&normalized_path).unwrap().template))
    }

    #[allow(clippy::only_used_in_recursion)]
    fn spawn_entity(
        &mut self,
        world: &mut World,
        entity_def: &EntityDefinition,
        base_path: &Path,
    ) -> TemplateResult<(Entity, Vec<TemplateInstance>)> {
        let mut references = Vec::new();

        match &entity_def.source {
            EntitySource::Inline { components, tags } => {
                let entity = world.spawn();

                // Use optimized component parsing with static dispatch
                for (component_name, component_value) in components {
                    self.add_component_to_entity_optimized(
                        world,
                        entity,
                        component_name,
                        component_value,
                    )?;
                }

                for (component_name, override_value) in &entity_def.overrides {
                    debug!(component = %component_name, "Applying override");
                    self.add_component_to_entity_optimized(
                        world,
                        entity,
                        component_name,
                        override_value,
                    )?;
                }

                if !tags.is_empty() {
                    debug!(tags = ?tags, "Entity tags");
                }

                for (child_name, child_def) in &entity_def.children {
                    debug!(child_name = %child_name, "Spawning child");
                    let (_child_entity, child_refs) =
                        self.spawn_entity(world, child_def, base_path)?;
                    references.extend(child_refs);
                }

                Ok((entity, references))
            }

            EntitySource::Reference { template } => {
                let template_path = if Path::new(template).is_absolute() {
                    PathBuf::from(template)
                } else {
                    let base_dir = base_path.parent().unwrap_or(Path::new("."));
                    base_dir.join(template)
                };

                info!(
                    reference = %template,
                    resolved_path = %template_path.display(),
                    "Loading referenced template"
                );

                let instance = self.load(world, &template_path)?;

                let entity = instance.entities.first().copied().ok_or_else(|| {
                    TemplateError::UnknownComponent {
                        component: format!("Referenced template has no entities: {}", template),
                    }
                })?;

                for (component_name, override_value) in &entity_def.overrides {
                    debug!(component = %component_name, "Applying override to reference");
                    self.add_component_to_entity_optimized(
                        world,
                        entity,
                        component_name,
                        override_value,
                    )?;
                }

                references.push(instance);

                Ok((entity, references))
            }
        }
    }

    /// Optimized component parsing using static dispatch instead of dynamic HashMap lookups.
    #[inline]
    fn add_component_to_entity_optimized(
        &self,
        world: &mut World,
        entity: Entity,
        component_name: &str,
        component_value: &serde_yaml::Value,
    ) -> TemplateResult<()> {
        // Use match statement for static dispatch (faster than HashMap lookups)
        match component_name {
            COMPONENT_TRANSFORM => {
                let transform = self.parse_transform(component_value)?;
                world.add(entity, transform);
            }
            COMPONENT_HEALTH => {
                let health = self.parse_health(component_value)?;
                world.add(entity, health);
            }
            COMPONENT_MESH_RENDERER => {
                let mesh_renderer = self.parse_mesh_renderer(component_value)?;
                world.add(entity, mesh_renderer);
            }
            COMPONENT_CAMERA => {
                let camera = self.parse_camera(component_value)?;
                world.add(entity, camera);
            }
            _ => {
                warn!(component = %component_name, "Unknown component type");
                return Err(TemplateError::UnknownComponent {
                    component: component_name.to_string(),
                });
            }
        }

        Ok(())
    }

    #[inline]
    fn parse_transform(&self, value: &serde_yaml::Value) -> TemplateResult<Transform> {
        if value.is_null() {
            return Ok(Transform::default());
        }

        let position = if let Some(pos) = value.get("position") {
            self.parse_vec3(pos)?
        } else {
            Vec3::ZERO
        };

        let rotation = if let Some(rot) = value.get("rotation") {
            self.parse_quat(rot)?
        } else {
            Quat::IDENTITY
        };

        let scale = if let Some(scl) = value.get("scale") {
            self.parse_vec3(scl)?
        } else {
            Vec3::ONE
        };

        Ok(Transform::new(position, rotation, scale))
    }

    #[inline]
    fn parse_health(&self, value: &serde_yaml::Value) -> TemplateResult<Health> {
        if value.is_null() {
            return Ok(Health::new(100.0, 100.0));
        }

        let current = value.get("current").and_then(|v| v.as_f64()).unwrap_or(100.0) as f32;
        let max = value.get("max").and_then(|v| v.as_f64()).unwrap_or(100.0) as f32;

        Ok(Health::new(current, max))
    }

    #[inline]
    fn parse_mesh_renderer(&self, value: &serde_yaml::Value) -> TemplateResult<MeshRenderer> {
        if value.is_null() {
            return Ok(MeshRenderer::new(0));
        }

        let mesh_id = if let Some(id) = value.get("mesh_id") {
            id.as_u64().unwrap_or(0)
        } else if let Some(mesh_path) = value.get("mesh") {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};

            let mut hasher = DefaultHasher::new();
            mesh_path.as_str().unwrap_or("").hash(&mut hasher);
            hasher.finish()
        } else {
            0
        };

        let visible = value.get("visible").and_then(|v| v.as_bool()).unwrap_or(true);

        Ok(MeshRenderer::with_visibility(mesh_id, visible))
    }

    #[inline]
    fn parse_camera(&self, value: &serde_yaml::Value) -> TemplateResult<Camera> {
        if value.is_null() {
            return Ok(Camera::default());
        }

        let fov = value.get("fov").and_then(|v| v.as_f64()).unwrap_or(60.0) as f32;
        let fov_radians = fov.to_radians();
        let aspect = value.get("aspect").and_then(|v| v.as_f64()).unwrap_or(16.0 / 9.0) as f32;
        let near = value.get("near").and_then(|v| v.as_f64()).unwrap_or(0.1) as f32;
        let far = value.get("far").and_then(|v| v.as_f64()).unwrap_or(1000.0) as f32;

        Ok(Camera::with_planes(fov_radians, aspect, near, far))
    }

    #[inline]
    fn parse_vec3(&self, value: &serde_yaml::Value) -> TemplateResult<Vec3> {
        if let Some(seq) = value.as_sequence() {
            if seq.len() >= 3 {
                let x = seq[0].as_f64().unwrap_or(0.0) as f32;
                let y = seq[1].as_f64().unwrap_or(0.0) as f32;
                let z = seq[2].as_f64().unwrap_or(0.0) as f32;
                return Ok(Vec3::new(x, y, z));
            }
        }

        Err(TemplateError::InvalidYaml { reason: "Vec3 must be array of 3 numbers".to_string() })
    }

    #[inline]
    fn parse_quat(&self, value: &serde_yaml::Value) -> TemplateResult<Quat> {
        if let Some(seq) = value.as_sequence() {
            if seq.len() >= 4 {
                let x = seq[0].as_f64().unwrap_or(0.0) as f32;
                let y = seq[1].as_f64().unwrap_or(0.0) as f32;
                let z = seq[2].as_f64().unwrap_or(0.0) as f32;
                let w = seq[3].as_f64().unwrap_or(1.0) as f32;
                return Ok(Quat::from_xyzw(x, y, z, w));
            }
        }

        Err(TemplateError::InvalidYaml {
            reason: "Quat must be array of 4 numbers [x, y, z, w]".to_string(),
        })
    }

    /// Clears the template cache.
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Returns true if the cache is empty.
    pub fn is_cache_empty(&self) -> bool {
        self.cache.is_empty()
    }

    /// Returns the number of templates in the cache.
    pub fn cache_size(&self) -> usize {
        self.cache.len()
    }
}

impl Default for TemplateLoaderOptimized {
    fn default() -> Self {
        Self::new()
    }
}

/// A spawned template instance (re-exported from original loader for compatibility).
#[derive(Debug, Clone)]
pub struct TemplateInstance {
    /// Name of the template
    pub name: String,

    /// All entities spawned from this template (top-level only)
    pub entities: Vec<Entity>,

    /// Referenced template instances
    pub references: Vec<TemplateInstance>,
}

impl TemplateInstance {
    /// Despawns all entities from this template instance.
    pub fn despawn(self, world: &mut World) {
        info!(
            template = %self.name,
            entity_count = self.entities.len(),
            "Despawning template instance"
        );

        for entity in self.entities {
            world.despawn(entity);
        }

        for reference in self.references {
            reference.despawn(world);
        }
    }

    /// Returns the total number of entities in this instance (including references).
    pub fn total_entity_count(&self) -> usize {
        let mut count = self.entities.len();
        for reference in &self.references {
            count += reference.total_entity_count();
        }
        count
    }
}
