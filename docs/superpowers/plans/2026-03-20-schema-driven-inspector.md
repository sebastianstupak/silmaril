# Schema-Driven Inspector Editing — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the hardcoded Inspector stub with a schema-driven property editor where component field definitions come from the engine's module registry, and appropriate widgets (slider, toggle, xyz inputs, etc.) are auto-rendered for each field type.

**Architecture:** The Rust backend hosts a `ComponentSchemaRegistry` that maps component type names to their field definitions (type, constraints, label). A new `get_component_schemas` Tauri command exposes the registry as JSON. The TypeScript frontend loads schemas once at startup, caches them, then uses them to render type-appropriate widgets for the selected entity's components. `SceneEntity` gains a `componentValues` bag for arbitrary component field values.

**Tech Stack:** Rust (schema registry, Tauri commands, serde_json), TypeScript/Vitest (schema types, inspector utils, TDD), Svelte 5 (widget components, InspectorPanel rewrite)

---

## File Map

### New Files

| Path | Responsibility |
|------|---------------|
| `src-tauri/bridge/schema_registry.rs` | `FieldType`, `FieldSchema`, `ComponentSchema`, `ComponentSchemaRegistry` |
| `src-tauri/bridge/builtin_schemas.rs` | Registers Transform, Health, Velocity, Camera, MeshRenderer, Collider |
| `src/lib/inspector/schema.ts` | TypeScript mirror of Rust schema types |
| `src/lib/inspector/inspector-utils.ts` | Pure functions: default values, applying defaults, building componentValues |
| `src/lib/inspector/inspector-utils.test.ts` | TDD tests for inspector-utils |
| `src/lib/inspector/schema-store.ts` | Loads + caches schemas once; pub/sub for components |
| `src/lib/inspector/widgets/F32Field.svelte` | Number input + optional range slider |
| `src/lib/inspector/widgets/BoolField.svelte` | Toggle/checkbox |
| `src/lib/inspector/widgets/Vec3Field.svelte` | Three labeled F32Fields (X/Y/Z) |
| `src/lib/inspector/widgets/EnumField.svelte` | Select dropdown |
| `src/lib/inspector/widgets/StringField.svelte` | Text input |

### Modified Files

| Path | Change |
|------|--------|
| `src-tauri/bridge/mod.rs` | Add `pub mod schema_registry; pub mod builtin_schemas;` |
| `src-tauri/bridge/commands.rs` | Add `ComponentSchemaState`, `get_component_schemas`, `set_component_field` |
| `src-tauri/lib.rs` | Init registry with builtins, manage as Tauri state, register new commands |
| `src/lib/api.ts` | Add `getComponentSchemas()`, `setComponentField()`, browser mocks |
| `src/lib/scene/state.ts` | Add `componentValues: EntityComponentValues` to `SceneEntity` |
| `src/lib/scene/commands.ts` | Init `componentValues` in `createEntity`/`populateFromScan`; add `setComponentField` |
| `src/lib/docking/panels/InspectorWrapper.svelte` | Update type annotation from `EntityInfo` to `SceneEntity` |
| `src/lib/components/InspectorPanel.svelte` | Full rewrite — schema-driven, widget-per-field |

---

## Task 1: ComponentSchemaRegistry

**Files:**
- Create: `src-tauri/bridge/schema_registry.rs`

- [ ] **Step 1: Write the failing tests**

```rust
// At the bottom of src-tauri/bridge/schema_registry.rs (write the whole file including these)
#[cfg(test)]
mod tests {
    use super::*;

    fn make_f32_field(name: &str) -> FieldSchema {
        FieldSchema {
            name: name.into(),
            label: name.into(),
            field_type: FieldType::F32 { min: None, max: None, step: None },
        }
    }

    #[test]
    fn test_register_and_get() {
        let mut reg = ComponentSchemaRegistry::new();
        reg.register(ComponentSchema {
            name: "Health".into(),
            label: "Health".into(),
            category: "Core".into(),
            fields: vec![make_f32_field("current")],
        });
        assert!(reg.get("Health").is_some());
        assert!(reg.get("Missing").is_none());
    }

    #[test]
    fn test_all_returns_all_registered() {
        let mut reg = ComponentSchemaRegistry::new();
        reg.register(ComponentSchema { name: "A".into(), label: "A".into(), category: "x".into(), fields: vec![] });
        reg.register(ComponentSchema { name: "B".into(), label: "B".into(), category: "x".into(), fields: vec![] });
        assert_eq!(reg.all().len(), 2);
    }

    #[test]
    fn test_empty_registry_all() {
        let reg = ComponentSchemaRegistry::new();
        assert_eq!(reg.all().len(), 0);
    }

    #[test]
    fn test_register_overwrites_existing() {
        let mut reg = ComponentSchemaRegistry::new();
        reg.register(ComponentSchema { name: "X".into(), label: "Old".into(), category: "c".into(), fields: vec![] });
        reg.register(ComponentSchema { name: "X".into(), label: "New".into(), category: "c".into(), fields: vec![] });
        assert_eq!(reg.get("X").unwrap().label, "New");
    }
}
```

- [ ] **Step 2: Run tests to confirm they FAIL**

```bash
cd engine/editor && cargo test -p silmaril-editor schema_registry 2>&1 | grep -E "error|FAILED"
```

Expected: compile error — types not yet defined

- [ ] **Step 3: Write the implementation** (above the tests in the same file)

```rust
//! Component schema registry — tracks field definitions for all component types.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The kind of a component field — drives which widget the inspector renders.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FieldType {
    F32 {
        min: Option<f32>,
        max: Option<f32>,
        step: Option<f32>,
    },
    Bool,
    String,
    Vec3,
    Enum {
        options: Vec<String>,
    },
}

/// Schema for a single field within a component.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldSchema {
    /// Internal field name (key in componentValues on the frontend).
    pub name: String,
    /// Human-readable label shown in the inspector.
    pub label: String,
    pub field_type: FieldType,
}

/// Full schema for one component type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentSchema {
    /// Type name as it appears in EntityInfo.components (e.g. "Transform").
    pub name: String,
    /// Human-readable display name.
    pub label: String,
    /// UI category for grouping (e.g. "Core", "Physics", "Rendering").
    pub category: String,
    pub fields: Vec<FieldSchema>,
}

/// Registry of all known component schemas.
#[derive(Default)]
pub struct ComponentSchemaRegistry {
    schemas: HashMap<String, ComponentSchema>,
}

impl ComponentSchemaRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers (or overwrites) a component schema.
    pub fn register(&mut self, schema: ComponentSchema) {
        self.schemas.insert(schema.name.clone(), schema);
    }

    /// Looks up a component by type name.
    pub fn get(&self, name: &str) -> Option<&ComponentSchema> {
        self.schemas.get(name)
    }

    /// Returns all registered schemas.
    pub fn all(&self) -> Vec<&ComponentSchema> {
        self.schemas.values().collect()
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cd engine/editor && cargo test -p silmaril-editor schema_registry 2>&1 | grep -E "test .* (ok|FAILED)|error"
```

Expected: `4 tests ok`

- [ ] **Step 5: Commit**

```bash
git add engine/editor/src-tauri/bridge/schema_registry.rs
git commit -m "feat(editor): add ComponentSchemaRegistry with FieldType enum"
```

---

## Task 2: Builtin Component Schemas

**Files:**
- Create: `src-tauri/bridge/builtin_schemas.rs`

- [ ] **Step 1: Write the failing tests** (end of the file)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::bridge::schema_registry::ComponentSchemaRegistry;

    fn registry_with_builtins() -> ComponentSchemaRegistry {
        let mut reg = ComponentSchemaRegistry::new();
        register_builtin_schemas(&mut reg);
        reg
    }

    #[test]
    fn test_transform_has_three_vec3_fields() {
        let reg = registry_with_builtins();
        let t = reg.get("Transform").expect("Transform not registered");
        assert_eq!(t.fields.len(), 3);
        assert!(t.fields.iter().any(|f| f.name == "position"));
        assert!(t.fields.iter().any(|f| f.name == "rotation"));
        assert!(t.fields.iter().any(|f| f.name == "scale"));
    }

    #[test]
    fn test_health_current_has_range() {
        use crate::bridge::schema_registry::FieldType;
        let reg = registry_with_builtins();
        let h = reg.get("Health").expect("Health not registered");
        let current = h.fields.iter().find(|f| f.name == "current").unwrap();
        if let FieldType::F32 { min, max, .. } = &current.field_type {
            assert_eq!(*min, Some(0.0));
            assert_eq!(*max, Some(10000.0));
        } else {
            panic!("expected F32");
        }
    }

    #[test]
    fn test_all_builtins_have_nonempty_category() {
        let reg = registry_with_builtins();
        for schema in reg.all() {
            assert!(!schema.category.is_empty(), "{} missing category", schema.name);
        }
    }

    #[test]
    fn test_six_builtins_registered() {
        let reg = registry_with_builtins();
        // Transform, Health, Velocity, Camera, MeshRenderer, Collider
        assert_eq!(reg.all().len(), 6);
    }
}
```

- [ ] **Step 2: Run tests to confirm they FAIL**

```bash
cd engine/editor && cargo test -p silmaril-editor builtin_schemas 2>&1 | grep -E "error|FAILED"
```

Expected: compile error — `register_builtin_schemas` not yet defined

- [ ] **Step 3: Write the implementation**

```rust
//! Built-in component schema registrations for core engine components.
//!
//! New modules register their own schemas via the EditorPlugin trait (future).
//! These cover the components that ship with the engine itself.

use super::schema_registry::{ComponentSchema, ComponentSchemaRegistry, FieldSchema, FieldType};

fn f32_field(name: &str, label: &str, min: Option<f32>, max: Option<f32>) -> FieldSchema {
    FieldSchema {
        name: name.into(),
        label: label.into(),
        field_type: FieldType::F32 { min, max, step: None },
    }
}

fn vec3_field(name: &str, label: &str) -> FieldSchema {
    FieldSchema {
        name: name.into(),
        label: label.into(),
        field_type: FieldType::Vec3,
    }
}

fn bool_field(name: &str, label: &str) -> FieldSchema {
    FieldSchema {
        name: name.into(),
        label: label.into(),
        field_type: FieldType::Bool,
    }
}

/// Registers all built-in engine component schemas.
pub fn register_builtin_schemas(registry: &mut ComponentSchemaRegistry) {
    registry.register(ComponentSchema {
        name: "Transform".into(),
        label: "Transform".into(),
        category: "Core".into(),
        fields: vec![
            vec3_field("position", "Position"),
            vec3_field("rotation", "Rotation"),
            vec3_field("scale", "Scale"),
        ],
    });

    registry.register(ComponentSchema {
        name: "Health".into(),
        label: "Health".into(),
        category: "Core".into(),
        fields: vec![
            f32_field("current", "Current HP", Some(0.0), Some(10000.0)),
            f32_field("max", "Max HP", Some(1.0), Some(10000.0)),
        ],
    });

    registry.register(ComponentSchema {
        name: "Velocity".into(),
        label: "Velocity".into(),
        category: "Physics".into(),
        fields: vec![
            vec3_field("linear", "Linear"),
            vec3_field("angular", "Angular"),
        ],
    });

    registry.register(ComponentSchema {
        name: "Camera".into(),
        label: "Camera".into(),
        category: "Rendering".into(),
        fields: vec![
            f32_field("fov", "Field of View", Some(1.0), Some(180.0)),
            f32_field("near", "Near Clip", Some(0.001), Some(10.0)),
            f32_field("far", "Far Clip", Some(1.0), Some(100_000.0)),
        ],
    });

    registry.register(ComponentSchema {
        name: "MeshRenderer".into(),
        label: "Mesh Renderer".into(),
        category: "Rendering".into(),
        fields: vec![
            bool_field("visible", "Visible"),
            bool_field("cast_shadows", "Cast Shadows"),
            bool_field("receive_shadows", "Receive Shadows"),
        ],
    });

    registry.register(ComponentSchema {
        name: "Collider".into(),
        label: "Collider".into(),
        category: "Physics".into(),
        fields: vec![
            bool_field("is_trigger", "Is Trigger"),
            f32_field("friction", "Friction", Some(0.0), Some(1.0)),
            f32_field("restitution", "Restitution", Some(0.0), Some(1.0)),
        ],
    });
}
```

- [ ] **Step 4: Expose modules in bridge/mod.rs**

Add to `src-tauri/bridge/mod.rs`:
```rust
pub mod builtin_schemas;
pub mod schema_registry;
```

- [ ] **Step 5: Run the tests**

```bash
cd engine/editor && cargo test -p silmaril-editor builtin_schemas 2>&1 | grep -E "test .* (ok|FAILED)|error"
```

Expected: `4 tests ok`

- [ ] **Step 6: Commit**

```bash
git add engine/editor/src-tauri/bridge/schema_registry.rs \
        engine/editor/src-tauri/bridge/builtin_schemas.rs \
        engine/editor/src-tauri/bridge/mod.rs
git commit -m "feat(editor): builtin component schemas (Transform, Health, Velocity, Camera, MeshRenderer, Collider)"
```

---

## Task 3: Tauri Commands — get_component_schemas + set_component_field

**Files:**
- Modify: `src-tauri/bridge/commands.rs`
- Modify: `src-tauri/lib.rs`

- [ ] **Step 1: Add ComponentSchemaState and commands to commands.rs**

At the top of `commands.rs`, after existing imports, add:
```rust
use crate::bridge::schema_registry::ComponentSchemaRegistry;
```

After the existing state structs (near `NativeViewportState`), add:
```rust
/// Tauri managed state holding the component schema registry.
pub struct ComponentSchemaState(pub std::sync::Mutex<ComponentSchemaRegistry>);
```

Add these two commands anywhere before the existing commands (e.g., after `get_editor_state`):
```rust
/// Returns all registered component schemas.
///
/// `ComponentSchema` derives `Serialize`, so Tauri serializes it directly —
/// no intermediate `serde_json::Value` step needed.
#[tauri::command]
pub fn get_component_schemas(
    state: tauri::State<ComponentSchemaState>,
) -> Result<Vec<ComponentSchema>, String> {
    let registry = state.0.lock().map_err(|e| e.to_string())?;
    Ok(registry.all().into_iter().cloned().collect())
}

/// Sets a single component field value for an entity.
///
/// Design-time: updates are tracked in the frontend scene state.
/// Play-time (future): this will forward to the live ECS.
#[tauri::command]
pub fn set_component_field(
    entity_id: u64,
    component: String,
    field: String,
    value: serde_json::Value,
) -> Result<(), String> {
    tracing::debug!(
        entity_id,
        component = %component,
        field = %field,
        value = %value,
        "set_component_field"
    );
    Ok(())
}
```

- [ ] **Step 2: Wire into lib.rs**

In `lib.rs`, add import at the top:
```rust
use crate::bridge::{
    builtin_schemas::register_builtin_schemas,
    commands::ComponentSchemaState,
    schema_registry::ComponentSchemaRegistry,
};
```

In the Tauri builder `.setup()` closure, after the existing state setup, add:
```rust
let mut schema_registry = ComponentSchemaRegistry::new();
register_builtin_schemas(&mut schema_registry);
app.manage(ComponentSchemaState(std::sync::Mutex::new(schema_registry)));
```

In `.invoke_handler(tauri::generate_handler![...])`, add `get_component_schemas` and `set_component_field` to the list.

- [ ] **Step 3: Verify it compiles**

```bash
cd engine/editor && cargo build -p silmaril-editor 2>&1 | grep -E "^error" | head -20
```

Expected: no output (clean build)

- [ ] **Step 4: Commit**

```bash
git add engine/editor/src-tauri/bridge/commands.rs engine/editor/src-tauri/lib.rs
git commit -m "feat(editor): get_component_schemas and set_component_field Tauri commands"
```

---

## Task 4: TypeScript Schema Types + API Extension

**Files:**
- Create: `src/lib/inspector/schema.ts`
- Modify: `src/lib/api.ts`

- [ ] **Step 1: Create schema.ts**

```typescript
// src/lib/inspector/schema.ts
// TypeScript mirror of Rust's FieldType / FieldSchema / ComponentSchema.
// Keep in sync with src-tauri/bridge/schema_registry.rs.

export type FieldType =
  | { kind: 'f32'; min?: number; max?: number; step?: number }
  | { kind: 'bool' }
  | { kind: 'string' }
  | { kind: 'vec3' }
  | { kind: 'enum'; options: string[] };

export interface FieldSchema {
  name: string;
  label: string;
  field_type: FieldType;
}

export interface ComponentSchema {
  name: string;
  label: string;
  category: string;
  fields: FieldSchema[];
}

/** Map from component type name to its schema. */
export type ComponentSchemas = Record<string, ComponentSchema>;
```

- [ ] **Step 2: Add to api.ts**

Add the import at the **top** of `api.ts` (with the other imports):
```typescript
import type { ComponentSchema } from '$lib/inspector/schema';
```

Add the mock data and functions at the **bottom** of `api.ts`:

```typescript
// --- mock data ---
const mockSchemas: ComponentSchema[] = [
  {
    name: 'Transform',
    label: 'Transform',
    category: 'Core',
    fields: [
      { name: 'position', label: 'Position', field_type: { kind: 'vec3' } },
      { name: 'rotation', label: 'Rotation', field_type: { kind: 'vec3' } },
      { name: 'scale',    label: 'Scale',    field_type: { kind: 'vec3' } },
    ],
  },
  {
    name: 'Health',
    label: 'Health',
    category: 'Core',
    fields: [
      { name: 'current', label: 'Current HP', field_type: { kind: 'f32', min: 0, max: 10000 } },
      { name: 'max',     label: 'Max HP',     field_type: { kind: 'f32', min: 1, max: 10000 } },
    ],
  },
];

// Add to the mocks object inside browserMock():
//   get_component_schemas: mockSchemas,
//   set_component_field: null,   ← null is correct for void-returning commands (matches existing pattern)

export async function getComponentSchemas(): Promise<ComponentSchema[]> {
  return tauriInvoke<ComponentSchema[]>('get_component_schemas');
}

export async function setComponentField(
  entityId: number,
  component: string,
  field: string,
  value: unknown,
): Promise<void> {
  return tauriInvoke<void>('set_component_field', { entityId, component, field, value });
}
```

Also add `get_component_schemas: mockSchemas` and `set_component_field: null` inside the `mocks` object in `browserMock()`.

- [ ] **Step 3: Verify TypeScript compiles**

```bash
cd engine/editor && npx tsc --noEmit 2>&1 | grep error | head -20
```

Expected: no output

- [ ] **Step 4: Commit**

```bash
git add engine/editor/src/lib/inspector/schema.ts engine/editor/src/lib/api.ts
git commit -m "feat(editor): TypeScript schema types and API functions for component schemas"
```

---

## Task 5: Inspector Utils (TDD)

**Files:**
- Create: `src/lib/inspector/inspector-utils.test.ts` (RED first)
- Create: `src/lib/inspector/inspector-utils.ts` (GREEN)

- [ ] **Step 1: Write the failing tests**

```typescript
// src/lib/inspector/inspector-utils.test.ts
import { describe, it, expect } from 'vitest';
import {
  defaultValueForField,
  applyComponentDefaults,
  buildInitialComponentValues,
} from './inspector-utils';
import type { ComponentSchema } from './schema';

const healthSchema: ComponentSchema = {
  name: 'Health',
  label: 'Health',
  category: 'Core',
  fields: [
    { name: 'current', label: 'Current', field_type: { kind: 'f32', min: 0, max: 1000 } },
    { name: 'max',     label: 'Max',     field_type: { kind: 'f32', min: 1, max: 1000 } },
  ],
};

const transformSchema: ComponentSchema = {
  name: 'Transform',
  label: 'Transform',
  category: 'Core',
  fields: [
    { name: 'position', label: 'Position', field_type: { kind: 'vec3' } },
    { name: 'rotation', label: 'Rotation', field_type: { kind: 'vec3' } },
    { name: 'scale',    label: 'Scale',    field_type: { kind: 'vec3' } },
  ],
};

describe('defaultValueForField', () => {
  it('returns min for f32 with min set', () => {
    expect(defaultValueForField({ kind: 'f32', min: 5 })).toBe(5);
  });

  it('returns 0 for f32 with no min', () => {
    expect(defaultValueForField({ kind: 'f32' })).toBe(0);
  });

  it('returns false for bool', () => {
    expect(defaultValueForField({ kind: 'bool' })).toBe(false);
  });

  it('returns empty string for string', () => {
    expect(defaultValueForField({ kind: 'string' })).toBe('');
  });

  it('returns zero vec3 for vec3', () => {
    expect(defaultValueForField({ kind: 'vec3' })).toEqual({ x: 0, y: 0, z: 0 });
  });

  it('returns first option for enum', () => {
    expect(defaultValueForField({ kind: 'enum', options: ['A', 'B', 'C'] })).toBe('A');
  });

  it('returns empty string for enum with no options', () => {
    expect(defaultValueForField({ kind: 'enum', options: [] })).toBe('');
  });
});

describe('applyComponentDefaults', () => {
  it('fills all missing fields with defaults', () => {
    const values = applyComponentDefaults(healthSchema, {});
    expect(values.current).toBe(0);  // min of [0, 1000]
    expect(values.max).toBe(1);     // min of [1, 1000]
  });

  it('does not overwrite existing values', () => {
    const values = applyComponentDefaults(healthSchema, { current: 75 });
    expect(values.current).toBe(75);
    expect(values.max).toBe(1);
  });

  it('handles vec3 fields', () => {
    const values = applyComponentDefaults(transformSchema, {});
    expect(values.position).toEqual({ x: 0, y: 0, z: 0 });
  });

  it('returns copy, not mutation of input', () => {
    const input = { current: 50 };
    const values = applyComponentDefaults(healthSchema, input);
    expect(input).toEqual({ current: 50 }); // input unchanged
    expect(values.max).toBe(1); // new field added to copy
  });
});

describe('buildInitialComponentValues', () => {
  const schemas = {
    Transform: transformSchema,
    Health: healthSchema,
  };

  it('builds values for all known components', () => {
    const result = buildInitialComponentValues(['Transform', 'Health'], schemas);
    expect(result.Transform).toBeDefined();
    expect(result.Transform.position).toEqual({ x: 0, y: 0, z: 0 });
    expect(result.Health.current).toBe(0);
  });

  it('preserves existing values for known components', () => {
    const existing = { Health: { current: 80, max: 100 } };
    const result = buildInitialComponentValues(['Health'], schemas, existing);
    expect(result.Health.current).toBe(80);
    expect(result.Health.max).toBe(100);
  });

  it('keeps empty record for unknown components (no schema)', () => {
    const result = buildInitialComponentValues(['AI'], schemas);
    expect(result.AI).toEqual({});
  });

  it('uses existing values for unknown components when provided', () => {
    const existing = { AI: { behavior: 'patrol' } };
    const result = buildInitialComponentValues(['AI'], schemas, existing);
    expect(result.AI).toEqual({ behavior: 'patrol' });
  });
});
```

- [ ] **Step 2: Run tests, confirm they fail**

```bash
cd engine/editor && npm test -- inspector-utils 2>&1 | tail -10
```

Expected: `FAIL` — `inspector-utils.ts` does not exist yet

- [ ] **Step 3: Write the implementation**

```typescript
// src/lib/inspector/inspector-utils.ts
import type { ComponentSchema, FieldType } from './schema';

export type FieldValue =
  | number
  | boolean
  | string
  | { x: number; y: number; z: number };

export type ComponentValues = Record<string, FieldValue>;
export type EntityComponentValues = Record<string, ComponentValues>;

/** Returns the default value for a given field type. */
export function defaultValueForField(ft: FieldType): FieldValue {
  switch (ft.kind) {
    case 'f32':    return ft.min ?? 0;
    case 'bool':   return false;
    case 'string': return '';
    case 'vec3':   return { x: 0, y: 0, z: 0 };
    case 'enum':   return ft.options[0] ?? '';
  }
}

/**
 * Returns a new ComponentValues with defaults applied for any fields missing
 * from `existing`. Does not mutate `existing`.
 */
export function applyComponentDefaults(
  schema: ComponentSchema,
  existing: ComponentValues = {},
): ComponentValues {
  const result: ComponentValues = { ...existing };
  for (const field of schema.fields) {
    if (!(field.name in result)) {
      result[field.name] = defaultValueForField(field.field_type);
    }
  }
  return result;
}

/**
 * Builds the full `componentValues` map for an entity.
 * - Known components: fills missing fields from schema defaults
 * - Unknown components (no schema): preserves existing values or empty record
 */
export function buildInitialComponentValues(
  componentNames: string[],
  schemas: Record<string, ComponentSchema>,
  existing: EntityComponentValues = {},
): EntityComponentValues {
  const result: EntityComponentValues = {};
  for (const name of componentNames) {
    const schema = schemas[name];
    if (schema) {
      result[name] = applyComponentDefaults(schema, existing[name]);
    } else {
      result[name] = existing[name] ?? {};
    }
  }
  return result;
}
```

- [ ] **Step 4: Run tests, confirm they pass**

```bash
cd engine/editor && npm test -- inspector-utils 2>&1 | tail -10
```

Expected: all tests pass

- [ ] **Step 5: Commit**

```bash
git add engine/editor/src/lib/inspector/inspector-utils.ts \
        engine/editor/src/lib/inspector/inspector-utils.test.ts
git commit -m "feat(editor): inspector utils with TDD (defaultValueForField, applyComponentDefaults, buildInitialComponentValues)"
```

---

## Task 6: Extend SceneEntity + Commands

**Files:**
- Modify: `src/lib/scene/state.ts`
- Modify: `src/lib/scene/commands.ts`

- [ ] **Step 1: Extend SceneEntity in state.ts**

Add import at top of `state.ts`:
```typescript
import type { EntityComponentValues } from '$lib/inspector/inspector-utils';
```

Add `componentValues` to the `SceneEntity` interface:
```typescript
export interface SceneEntity extends EntityInfo {
  position: Vec3;
  rotation: Vec3;
  scale: Vec3;
  visible: boolean;
  locked: boolean;
  /** Live field values for all components, keyed by component type name. */
  componentValues: EntityComponentValues;
}
```

- [ ] **Step 2: Update createEntity in commands.ts**

In `commands.ts`, add imports:
```typescript
import { buildInitialComponentValues } from '$lib/inspector/inspector-utils';
import { getSchemas } from '$lib/inspector/schema-store';
```

In `createEntity`, after building the entity object, initialize componentValues:
```typescript
// After: components: ['Transform']
componentValues: buildInitialComponentValues(
  ['Transform'],
  getSchemas(),
  {
    Transform: {
      position: { x: ..., y: 0, z: ... },  // use the random position
      rotation: { x: 0, y: 0, z: 0 },
      scale:    { x: 1, y: 1, z: 1 },
    },
  },
),
```

In `populateFromScan`, for each entity created from scan results:
```typescript
componentValues: buildInitialComponentValues(entity.components, getSchemas()),
```

- [ ] **Step 3: Add setComponentField to commands.ts**

```typescript
import { setComponentField as apiSetComponentField } from '$lib/api';

export function setComponentField(
  entityId: number,
  componentName: string,
  fieldName: string,
  value: unknown,
): void {
  _mutate((s) => ({
    ...s,
    entities: s.entities.map((e) => {
      if (e.id !== entityId) return e;
      return {
        ...e,
        componentValues: {
          ...e.componentValues,
          [componentName]: {
            ...e.componentValues[componentName],
            [fieldName]: value,
          },
        },
      };
    }),
  }));

  // Sync inline transform fields when a Transform field changes.
  // Branch on fieldName — do NOT spread objects into function args (objects are
  // not iterable; spreading { x, y, z } produces nothing).
  if (componentName === 'Transform') {
    const v = value as { x: number; y: number; z: number };
    if (fieldName === 'position') moveEntity(entityId, v.x, v.y, v.z);
    else if (fieldName === 'rotation') rotateEntity(entityId, v.x, v.y, v.z);
    else if (fieldName === 'scale') scaleEntity(entityId, v.x, v.y, v.z);
  }

  // Forward to Tauri (no-op in browser; will wire to live ECS in Play mode)
  apiSetComponentField(entityId, componentName, fieldName, value).catch(() => {});
}
```

**Note — `duplicateEntity` requires no changes.** It uses `structuredClone(source)` which deep-copies `componentValues` correctly since all field values are plain objects or primitives.

**Note — race condition with `getSchemas()`.** `loadSchemas()` is async. If `populateFromScan` runs before schemas have loaded, `getSchemas()` returns `{}` and entities get empty `componentValues`. This is acceptable for MVP: the inspector shows gracefully degraded state, and values will be correct once the user re-selects an entity after schemas have loaded.

- [ ] **Step 4: Verify TypeScript compiles**

```bash
cd engine/editor && npx tsc --noEmit 2>&1 | grep error | head -20
```

Expected: no output

- [ ] **Step 5: Commit**

```bash
git add engine/editor/src/lib/scene/state.ts engine/editor/src/lib/scene/commands.ts
git commit -m "feat(editor): add componentValues to SceneEntity and setComponentField command"
```

---

## Task 7: Schema Store

**Files:**
- Create: `src/lib/inspector/schema-store.ts`

- [ ] **Step 1: Create the schema store** (no TDD — it's a side-effect loader)

```typescript
// src/lib/inspector/schema-store.ts
// Loads component schemas from Tauri once and caches them.
// Follows the same pub/sub pattern as scene/state.ts.

import type { ComponentSchemas } from './schema';
import { getComponentSchemas } from '$lib/api';

let schemas: ComponentSchemas = {};
let loaded = false;
const listeners: Array<() => void> = [];

function notify() {
  for (const fn of listeners) fn();
}

/** Load schemas from the backend (idempotent — safe to call multiple times). */
export async function loadSchemas(): Promise<void> {
  if (loaded) return;
  const list = await getComponentSchemas();
  schemas = Object.fromEntries(list.map((s) => [s.name, s]));
  loaded = true;
  notify();
}

/** Get the cached schema map. Returns empty object before loadSchemas() resolves. */
export function getSchemas(): ComponentSchemas {
  return schemas;
}

/** Subscribe to schema updates. Returns unsubscribe function. */
export function subscribeSchemas(fn: () => void): () => void {
  listeners.push(fn);
  return () => {
    const i = listeners.indexOf(fn);
    if (i >= 0) listeners.splice(i, 1);
  };
}
```

- [ ] **Step 2: Call loadSchemas on app startup in App.svelte**

In the `onMount` of `App.svelte`, add:
```typescript
import { loadSchemas } from '$lib/inspector/schema-store';

// inside onMount, alongside other hydration calls:
loadSchemas(); // fire-and-forget; store notifies subscribers when ready
```

- [ ] **Step 3: Verify TypeScript compiles**

```bash
cd engine/editor && npx tsc --noEmit 2>&1 | grep error | head -20
```

- [ ] **Step 4: Commit**

```bash
git add engine/editor/src/lib/inspector/schema-store.ts engine/editor/src/App.svelte
git commit -m "feat(editor): schema store — loads and caches component schemas on startup"
```

---

## Task 8: Inspector Widget Components

**Files:**
- Create: `src/lib/inspector/widgets/F32Field.svelte`
- Create: `src/lib/inspector/widgets/BoolField.svelte`
- Create: `src/lib/inspector/widgets/Vec3Field.svelte`
- Create: `src/lib/inspector/widgets/EnumField.svelte`
- Create: `src/lib/inspector/widgets/StringField.svelte`

Each widget receives `label`, `value`, and fires `onchange(newValue)`.

- [ ] **Step 1: Create F32Field.svelte**

```svelte
<!-- src/lib/inspector/widgets/F32Field.svelte -->
<script lang="ts">
  let {
    label,
    value = 0,
    min,
    max,
    step = 0.1,
    onchange,
  }: {
    label: string;
    value?: number;
    min?: number;
    max?: number;
    step?: number;
    onchange?: (v: number) => void;
  } = $props();

  function handleInput(e: Event) {
    const v = parseFloat((e.target as HTMLInputElement).value);
    if (!isNaN(v)) onchange?.(v);
  }
</script>

<div class="field-row">
  <label class="field-label">{label}</label>
  <div class="field-controls">
    {#if min !== undefined && max !== undefined}
      <input
        type="range"
        {min}
        {max}
        {step}
        value={value}
        oninput={handleInput}
        class="field-slider"
      />
    {/if}
    <input
      type="number"
      value={value}
      {step}
      oninput={handleInput}
      class="field-number"
    />
  </div>
</div>

<style>
  .field-row {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 2px 0;
    font-size: 11px;
  }
  .field-label {
    color: var(--color-textMuted, #999);
    min-width: 70px;
    flex-shrink: 0;
  }
  .field-controls {
    display: flex;
    align-items: center;
    gap: 4px;
    flex: 1;
    min-width: 0;
  }
  .field-slider {
    flex: 1;
    min-width: 0;
    accent-color: var(--color-accent, #007acc);
  }
  .field-number {
    width: 52px;
    flex-shrink: 0;
    background: var(--color-bgInput, #1a1a1a);
    border: 1px solid var(--color-border, #404040);
    border-radius: 3px;
    color: var(--color-text, #ccc);
    font-size: 11px;
    padding: 1px 4px;
    text-align: right;
  }
  .field-number:focus {
    outline: none;
    border-color: var(--color-accent, #007acc);
  }
</style>
```

- [ ] **Step 2: Create BoolField.svelte**

```svelte
<!-- src/lib/inspector/widgets/BoolField.svelte -->
<script lang="ts">
  let {
    label,
    value = false,
    onchange,
  }: {
    label: string;
    value?: boolean;
    onchange?: (v: boolean) => void;
  } = $props();
</script>

<div class="field-row">
  <label class="field-label">{label}</label>
  <input
    type="checkbox"
    checked={value}
    onchange={(e) => onchange?.((e.target as HTMLInputElement).checked)}
    class="field-checkbox"
  />
</div>

<style>
  .field-row {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 2px 0;
    font-size: 11px;
  }
  .field-label {
    color: var(--color-textMuted, #999);
    min-width: 70px;
    flex-shrink: 0;
  }
  .field-checkbox {
    accent-color: var(--color-accent, #007acc);
    cursor: pointer;
  }
</style>
```

- [ ] **Step 3: Create Vec3Field.svelte**

```svelte
<!-- src/lib/inspector/widgets/Vec3Field.svelte -->
<script lang="ts">
  import F32Field from './F32Field.svelte';

  let {
    label,
    value = { x: 0, y: 0, z: 0 },
    onchange,
  }: {
    label: string;
    value?: { x: number; y: number; z: number };
    onchange?: (v: { x: number; y: number; z: number }) => void;
  } = $props();

  function update(axis: 'x' | 'y' | 'z', v: number) {
    onchange?.({ ...value, [axis]: v });
  }
</script>

<div class="vec3-group">
  <div class="vec3-label">{label}</div>
  <div class="vec3-axes">
    <F32Field label="X" value={value.x} onchange={(v) => update('x', v)} />
    <F32Field label="Y" value={value.y} onchange={(v) => update('y', v)} />
    <F32Field label="Z" value={value.z} onchange={(v) => update('z', v)} />
  </div>
</div>

<style>
  .vec3-group {
    padding: 2px 0 4px;
  }
  .vec3-label {
    font-size: 11px;
    color: var(--color-textMuted, #999);
    margin-bottom: 2px;
  }
  .vec3-axes {
    display: flex;
    flex-direction: column;
    gap: 1px;
    padding-left: 8px;
  }
</style>
```

- [ ] **Step 4: Create EnumField.svelte**

```svelte
<!-- src/lib/inspector/widgets/EnumField.svelte -->
<script lang="ts">
  let {
    label,
    value = '',
    options = [],
    onchange,
  }: {
    label: string;
    value?: string;
    options?: string[];
    onchange?: (v: string) => void;
  } = $props();
</script>

<div class="field-row">
  <label class="field-label">{label}</label>
  <select
    value={value}
    onchange={(e) => onchange?.((e.target as HTMLSelectElement).value)}
    class="field-select"
  >
    {#each options as opt}
      <option value={opt}>{opt}</option>
    {/each}
  </select>
</div>

<style>
  .field-row {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 2px 0;
    font-size: 11px;
  }
  .field-label {
    color: var(--color-textMuted, #999);
    min-width: 70px;
    flex-shrink: 0;
  }
  .field-select {
    flex: 1;
    background: var(--color-bgInput, #1a1a1a);
    border: 1px solid var(--color-border, #404040);
    border-radius: 3px;
    color: var(--color-text, #ccc);
    font-size: 11px;
    padding: 1px 4px;
  }
</style>
```

- [ ] **Step 5: Create StringField.svelte**

```svelte
<!-- src/lib/inspector/widgets/StringField.svelte -->
<script lang="ts">
  let {
    label,
    value = '',
    onchange,
  }: {
    label: string;
    value?: string;
    onchange?: (v: string) => void;
  } = $props();
</script>

<div class="field-row">
  <label class="field-label">{label}</label>
  <input
    type="text"
    value={value}
    oninput={(e) => onchange?.((e.target as HTMLInputElement).value)}
    class="field-input"
  />
</div>

<style>
  .field-row {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 2px 0;
    font-size: 11px;
  }
  .field-label {
    color: var(--color-textMuted, #999);
    min-width: 70px;
    flex-shrink: 0;
  }
  .field-input {
    flex: 1;
    background: var(--color-bgInput, #1a1a1a);
    border: 1px solid var(--color-border, #404040);
    border-radius: 3px;
    color: var(--color-text, #ccc);
    font-size: 11px;
    padding: 1px 4px;
  }
  .field-input:focus {
    outline: none;
    border-color: var(--color-accent, #007acc);
  }
</style>
```

- [ ] **Step 6: Verify TypeScript compiles**

```bash
cd engine/editor && npx tsc --noEmit 2>&1 | grep error | head -20
```

- [ ] **Step 7: Commit**

```bash
git add engine/editor/src/lib/inspector/widgets/
git commit -m "feat(editor): Inspector widget components (F32Field, BoolField, Vec3Field, EnumField, StringField)"
```

---

## Task 9: Rewrite InspectorPanel

**Files:**
- Modify: `src/lib/components/InspectorPanel.svelte`

This replaces the hardcoded stub with schema-driven rendering.

- [ ] **Step 1: Rewrite InspectorPanel.svelte**

```svelte
<!-- src/lib/components/InspectorPanel.svelte -->
<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { t } from '$lib/i18n';
  import type { SceneEntity } from '$lib/scene/state';
  import { subscribeSchemas, getSchemas } from '$lib/inspector/schema-store';
  import { setComponentField } from '$lib/scene/commands';
  import type { ComponentSchemas } from '$lib/inspector/schema';
  import F32Field    from '$lib/inspector/widgets/F32Field.svelte';
  import BoolField   from '$lib/inspector/widgets/BoolField.svelte';
  import Vec3Field   from '$lib/inspector/widgets/Vec3Field.svelte';
  import EnumField   from '$lib/inspector/widgets/EnumField.svelte';
  import StringField from '$lib/inspector/widgets/StringField.svelte';

  let { entity = null }: { entity: SceneEntity | null } = $props();

  let schemas: ComponentSchemas = $state(getSchemas());
  let collapsedSections: Record<string, boolean> = $state({});

  // Refresh schemas when store loads (schemas arrive async after startup)
  const unsub = subscribeSchemas(() => { schemas = getSchemas(); });
  onDestroy(unsub);

  function toggleSection(name: string) {
    collapsedSections[name] = !collapsedSections[name];
  }

  function handleFieldChange(componentName: string, fieldName: string, value: unknown) {
    if (!entity) return;
    setComponentField(entity.id, componentName, fieldName, value);
  }

  function fieldValue(componentName: string, fieldName: string): unknown {
    return entity?.componentValues?.[componentName]?.[fieldName];
  }
</script>

<div class="inspector">
  {#if !entity}
    <p class="inspector-empty">{t('inspector.no_selection')}</p>
  {:else}
    <!-- Header -->
    <div class="inspector-header">
      <span class="inspector-entity-icon">
        <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
          <path d="M8 1l6.5 3.75v6.5L8 15l-6.5-3.75v-6.5L8 1z" stroke="currentColor" stroke-width="1" fill="none"/>
        </svg>
      </span>
      <span class="inspector-entity-name">{entity.name}</span>
      <span class="inspector-entity-id">#{entity.id}</span>
    </div>

    <div class="inspector-section-label">{t('inspector.components')}</div>

    <!-- Components -->
    {#each entity.components as componentName (componentName)}
      {@const schema = schemas[componentName]}
      <div class="component-section">
        <button
          class="component-header"
          onclick={() => toggleSection(componentName)}
          aria-expanded={!collapsedSections[componentName]}
        >
          <span class="component-chevron" class:collapsed={collapsedSections[componentName]}>
            <svg width="10" height="10" viewBox="0 0 16 16" fill="currentColor">
              <path d="M4 6l4 4 4-4" stroke="currentColor" stroke-width="1.5" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
          </span>
          <span class="component-name">{schema?.label ?? componentName}</span>
          {#if schema}
            <span class="component-category">{schema.category}</span>
          {/if}
        </button>

        {#if !collapsedSections[componentName]}
          <div class="component-body">
            {#if schema}
              {#each schema.fields as field (field.name)}
                {@const ft = field.field_type}
                {@const val = fieldValue(componentName, field.name)}
                <div class="field-wrapper">
                  {#if ft.kind === 'f32'}
                    <F32Field
                      label={field.label}
                      value={val as number ?? 0}
                      min={ft.min}
                      max={ft.max}
                      step={ft.step ?? 0.1}
                      onchange={(v) => handleFieldChange(componentName, field.name, v)}
                    />
                  {:else if ft.kind === 'bool'}
                    <BoolField
                      label={field.label}
                      value={val as boolean ?? false}
                      onchange={(v) => handleFieldChange(componentName, field.name, v)}
                    />
                  {:else if ft.kind === 'vec3'}
                    <Vec3Field
                      label={field.label}
                      value={val as { x: number; y: number; z: number } ?? { x: 0, y: 0, z: 0 }}
                      onchange={(v) => handleFieldChange(componentName, field.name, v)}
                    />
                  {:else if ft.kind === 'enum'}
                    <EnumField
                      label={field.label}
                      value={val as string ?? ''}
                      options={ft.options}
                      onchange={(v) => handleFieldChange(componentName, field.name, v)}
                    />
                  {:else if ft.kind === 'string'}
                    <StringField
                      label={field.label}
                      value={val as string ?? ''}
                      onchange={(v) => handleFieldChange(componentName, field.name, v)}
                    />
                  {:else}
                    <div class="field-row">
                      <span class="field-label">{field.label}</span>
                      <span class="field-value">{JSON.stringify(val)}</span>
                    </div>
                  {/if}
                </div>
              {/each}
            {:else}
              <!-- No schema registered — show raw values if any -->
              {#each Object.entries(entity.componentValues?.[componentName] ?? {}) as [k, v]}
                <div class="field-row">
                  <span class="field-label">{k}</span>
                  <span class="field-value">{JSON.stringify(v)}</span>
                </div>
              {/each}
              {#if !entity.componentValues?.[componentName] || Object.keys(entity.componentValues[componentName]).length === 0}
                <div class="field-row">
                  <span class="field-label field-label--dim">no schema</span>
                </div>
              {/if}
            {/if}
          </div>
        {/if}
      </div>
    {/each}

    <button class="add-component-btn">+ {t('inspector.add_component')}</button>
  {/if}
</div>

<style>
  .inspector {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow-y: auto;
  }

  .inspector-empty {
    color: var(--color-textDim, #666);
    font-style: italic;
    padding: 12px 8px;
    font-size: 12px;
    text-align: center;
  }

  .inspector-header {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px;
    border-bottom: 1px solid var(--color-border, #404040);
    flex-shrink: 0;
  }

  .inspector-entity-icon {
    display: flex;
    align-items: center;
    color: var(--color-accent, #007acc);
    flex-shrink: 0;
  }

  .inspector-entity-name {
    font-size: 13px;
    font-weight: 600;
    color: var(--color-text, #ccc);
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .inspector-entity-id {
    font-size: 10px;
    color: var(--color-textDim, #666);
    flex-shrink: 0;
  }

  .inspector-section-label {
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--color-textDim, #666);
    padding: 8px 8px 4px;
  }

  .component-section {
    border-bottom: 1px solid var(--color-border, #404040);
  }

  .component-header {
    display: flex;
    align-items: center;
    gap: 4px;
    width: 100%;
    padding: 6px 8px;
    background: var(--color-bgHeader, #2d2d2d);
    border: none;
    cursor: pointer;
    color: var(--color-text, #ccc);
    font-size: 12px;
    font-weight: 500;
    text-align: left;
  }

  .component-header:hover {
    background: var(--color-bg, #1e1e1e);
  }

  .component-chevron {
    display: flex;
    align-items: center;
    transition: transform 0.15s ease;
    color: var(--color-textMuted, #999);
  }

  .component-chevron.collapsed {
    transform: rotate(-90deg);
  }

  .component-name {
    flex: 1;
  }

  .component-category {
    font-size: 10px;
    color: var(--color-textDim, #555);
    font-weight: 400;
  }

  .component-body {
    padding: 4px 8px 8px 8px;
  }

  .field-wrapper {
    padding: 1px 0;
  }

  .field-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 2px 0;
    font-size: 11px;
  }

  .field-label {
    color: var(--color-textMuted, #999);
    min-width: 70px;
    flex-shrink: 0;
  }

  .field-label--dim {
    font-style: italic;
    color: var(--color-textDim, #555);
  }

  .field-value {
    color: var(--color-text, #ccc);
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: monospace;
    font-size: 10px;
  }

  .add-component-btn {
    margin: 8px;
    padding: 6px 12px;
    background: var(--color-bg, #1e1e1e);
    border: 1px dashed var(--color-border, #404040);
    border-radius: 4px;
    color: var(--color-textMuted, #999);
    font-size: 11px;
    cursor: pointer;
    text-align: center;
  }

  .add-component-btn:hover {
    border-color: var(--color-accent, #007acc);
    color: var(--color-accent, #007acc);
  }
</style>
```

- [ ] **Step 2: Update InspectorWrapper type annotation**

In `src/lib/docking/panels/InspectorWrapper.svelte`, make two changes:

1. Replace `import type { EntityInfo } from '$lib/api';` with `import type { SceneEntity } from '$lib/scene/state';`
2. Change `let entity: EntityInfo | null = $state(...)` to `let entity: SceneEntity | null = $state(...)`

`getSelectedEntity()` already returns `SceneEntity | null` at runtime — this is purely a type annotation fix. Once `SceneEntity.componentValues` is required, keeping `EntityInfo` will cause a TypeScript error.

- [ ] **Step 3: Run TypeScript check**

```bash
cd engine/editor && npx tsc --noEmit 2>&1 | grep error | head -20
```

Expected: no output

- [ ] **Step 4: Run all tests**

```bash
cd engine/editor && npm test 2>&1 | tail -5
```

Expected: all tests pass (count should be ≥ 96 + 17 new inspector-utils tests)

- [ ] **Step 5: Run Rust tests**

```bash
cd engine/editor && cargo test -p silmaril-editor 2>&1 | grep -E "test result|FAILED"
```

Expected: all pass

- [ ] **Step 6: Commit**

```bash
git add engine/editor/src/lib/components/InspectorPanel.svelte
git commit -m "feat(editor): schema-driven InspectorPanel — auto-renders widgets from component schemas"
```

---

## Verification Checklist

Before calling this complete:

- [ ] `npm test` passes (all tests including 16 new inspector-utils tests)
- [ ] `cargo test -p silmaril-editor` passes (8 new schema tests)
- [ ] `npx tsc --noEmit` — no errors
- [ ] `cargo clippy` — no errors
- [ ] Select an entity in the editor → Inspector shows component sections with real field widgets
- [ ] Edit a Transform position field → entity moves in the hierarchy (via setComponentField → moveEntity)
- [ ] Edit a Health field → value persists on re-select
- [ ] Entity with unknown component (e.g. "AI") → Inspector shows "no schema" gracefully
