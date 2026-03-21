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
