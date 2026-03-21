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
