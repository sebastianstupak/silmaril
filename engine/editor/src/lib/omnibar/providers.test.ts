// engine/editor/src/lib/omnibar/providers.test.ts
import { describe, it, expect } from 'vitest';
import { filterEntityResults, filterCommandResults, parsePrefix } from './providers';
import type { AnyCommand } from './types';

describe('parsePrefix', () => {
  it('detects > as command prefix', () => {
    expect(parsePrefix('>toggle')).toEqual({ prefix: 'command', query: 'toggle' });
  });

  it('detects @ as entity prefix', () => {
    expect(parsePrefix('@player')).toEqual({ prefix: 'entity', query: 'player' });
  });

  it('detects # as asset prefix', () => {
    expect(parsePrefix('#mesh')).toEqual({ prefix: 'asset', query: 'mesh' });
  });

  it('returns no prefix for plain query', () => {
    expect(parsePrefix('toggle')).toEqual({ prefix: null, query: 'toggle' });
  });

  it('handles prefix with space', () => {
    expect(parsePrefix('> toggle')).toEqual({ prefix: 'command', query: 'toggle' });
  });
});

describe('filterEntityResults', () => {
  const entities = [
    { id: 1, name: 'Player', components: ['Transform'] },
    { id: 2, name: 'Enemy', components: ['Transform', 'AI'] },
    { id: 3, name: 'PlayerCamera', components: ['Camera'] },
  ];

  it('returns all entities for empty query', () => {
    expect(filterEntityResults(entities, '')).toHaveLength(3);
  });

  it('filters by name fuzzy match', () => {
    const results = filterEntityResults(entities, 'player');
    expect(results.map(r => r.id)).toContain(1);
    expect(results.map(r => r.id)).toContain(3);
    expect(results.map(r => r.id)).not.toContain(2);
  });
});

describe('filterCommandResults', () => {
  const commands: AnyCommand[] = [
    { id: 'editor.toggle_grid', label: 'Toggle Grid', category: 'View' },
    { id: 'editor.toggle_snap', label: 'Toggle Snap', category: 'View' },
    { id: 'ui.open_settings', label: 'Open Settings', category: 'Editor', run: () => {} },
  ];

  it('returns all commands for empty query', () => {
    expect(filterCommandResults(commands, '')).toHaveLength(3);
  });

  it('filters by label fuzzy match', () => {
    const results = filterCommandResults(commands, 'tog');
    expect(results).toHaveLength(2);
  });
});
