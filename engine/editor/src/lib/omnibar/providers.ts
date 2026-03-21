// engine/editor/src/lib/omnibar/providers.ts
import type { AnyCommand, OmnibarResult } from './types';
import { fuzzyFilter } from './fuzzy';

export type PrefixType = 'command' | 'entity' | 'asset' | null;

export function parsePrefix(input: string): { prefix: PrefixType; query: string } {
  if (input.startsWith('>')) return { prefix: 'command', query: input.slice(1).trimStart() };
  if (input.startsWith('@')) return { prefix: 'entity', query: input.slice(1).trimStart() };
  if (input.startsWith('#')) return { prefix: 'asset', query: input.slice(1).trimStart() };
  return { prefix: null, query: input };
}

export function filterCommandResults(
  commands: AnyCommand[],
  query: string,
): AnyCommand[] {
  if (!query) return commands;
  return fuzzyFilter(commands, c => c.label, query).map(r => r.item);
}

export function filterEntityResults(
  entities: { id: number; name: string; components: string[] }[],
  query: string,
): { id: number; name: string; components: string[] }[] {
  if (!query) return entities;
  return fuzzyFilter(entities, e => e.name, query).map(r => r.item);
}

export function filterAssetResults(
  assets: { path: string; assetType: string }[],
  query: string,
): { path: string; assetType: string }[] {
  if (!query) return assets;
  return fuzzyFilter(assets, a => a.path, query).map(r => r.item);
}

/** Merge all providers into a flat OmnibarResult list respecting prefix routing. */
export function buildResults(
  query: string,
  commands: AnyCommand[],
  entities: { id: number; name: string; components: string[] }[],
  assets: { path: string; assetType: string }[],
  recent: { label: string; path: string; itemType: 'project' | 'scene' }[],
): OmnibarResult[] {
  const { prefix, query: q } = parsePrefix(query);
  const results: OmnibarResult[] = [];

  if (!prefix || prefix === 'command') {
    for (const cmd of filterCommandResults(commands, q)) {
      results.push({ kind: 'command', command: cmd });
    }
  }
  if (!prefix || prefix === 'entity') {
    for (const e of filterEntityResults(entities, q)) {
      results.push({ kind: 'entity', ...e });
    }
  }
  if (!prefix || prefix === 'asset') {
    for (const a of filterAssetResults(assets, q)) {
      results.push({ kind: 'asset', ...a });
    }
  }
  if (!prefix && !q) {
    // Empty input: show recent
    for (const r of recent) {
      results.push({ kind: 'recent', ...r });
    }
  }

  return results;
}
