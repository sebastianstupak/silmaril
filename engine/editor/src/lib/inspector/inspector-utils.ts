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
