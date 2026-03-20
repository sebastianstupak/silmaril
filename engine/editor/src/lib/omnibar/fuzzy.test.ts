import { describe, it, expect } from 'vitest';
import { fuzzyScore, fuzzyFilter } from './fuzzy';

describe('fuzzyScore', () => {
  it('returns 0 for empty query (matches everything)', () => {
    expect(fuzzyScore('Toggle Grid', '')).toBe(0);
  });

  it('scores exact match highest', () => {
    expect(fuzzyScore('Toggle Grid', 'Toggle Grid')).toBeGreaterThan(
      fuzzyScore('Toggle Grid', 'tog')
    );
  });

  it('is case insensitive', () => {
    expect(fuzzyScore('Toggle Grid', 'TOG')).toBeGreaterThan(-1);
  });

  it('returns -1 for no match', () => {
    expect(fuzzyScore('Toggle Grid', 'xyz')).toBe(-1);
  });

  it('prefix match scores higher than mid-string match', () => {
    expect(fuzzyScore('Toggle Grid', 'tog')).toBeGreaterThan(
      fuzzyScore('Toggle Grid', 'ggl')
    );
  });
});

describe('fuzzyFilter', () => {
  const items = ['Toggle Grid', 'Toggle Snap', 'Reset Camera', 'New Scene'];

  it('returns all items for empty query', () => {
    expect(fuzzyFilter(items, s => s, '')).toHaveLength(4);
  });

  it('filters items that do not match', () => {
    const results = fuzzyFilter(items, s => s, 'tog');
    expect(results.map(r => r.item)).toEqual(['Toggle Grid', 'Toggle Snap']);
  });

  it('sorts by score descending', () => {
    const results = fuzzyFilter(items, s => s, 'tog');
    expect(results[0].score).toBeGreaterThanOrEqual(results[1].score);
  });

  it('returns empty array when nothing matches', () => {
    expect(fuzzyFilter(items, s => s, 'zzz')).toHaveLength(0);
  });
});
