// engine/editor/src/lib/template/commands.test.ts
import { describe, it, expect, beforeEach } from 'vitest';
import { createEntity, addComponent, removeComponent, newScene } from './commands';
import { getEntityById } from './state';

// Reset template state before each test
beforeEach(() => { newScene(); });

describe('addComponent', () => {
  it('adds a component to an existing entity', () => {
    const e = createEntity('Tester');
    addComponent(e.id, 'Health');
    const updated = getEntityById(e.id);
    expect(updated?.components).toContain('Health');
  });

  it('is idempotent — adding same component twice does not duplicate', () => {
    const e = createEntity('Tester');
    addComponent(e.id, 'Health');
    addComponent(e.id, 'Health');
    const updated = getEntityById(e.id);
    const count = updated?.components.filter((c: string) => c === 'Health').length ?? 0;
    expect(count).toBe(1);
  });
});

describe('removeComponent', () => {
  it('removes a component from an entity', () => {
    const e = createEntity('Tester');
    addComponent(e.id, 'Health');
    removeComponent(e.id, 'Health');
    const updated = getEntityById(e.id);
    expect(updated?.components).not.toContain('Health');
  });

  it('is safe to call for a component not on the entity', () => {
    const e = createEntity('Tester');
    expect(() => removeComponent(e.id, 'NonExistent')).not.toThrow();
  });
});
