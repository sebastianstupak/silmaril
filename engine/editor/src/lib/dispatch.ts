import { commands, type CommandSpec } from './bindings';

// ──────────────────────────────────────────────────────────────────────────────
// Types
// ──────────────────────────────────────────────────────────────────────────────

export type CommandHandler = (args?: unknown) => Promise<void>;

export interface CommandRegistry {
  handlers: Map<string, CommandHandler>;
  specs: Map<string, CommandSpec>;
}

// ──────────────────────────────────────────────────────────────────────────────
// Module-level registry state
// ──────────────────────────────────────────────────────────────────────────────

const _registry: CommandRegistry = {
  handlers: new Map(),
  specs: new Map(),
};

// ──────────────────────────────────────────────────────────────────────────────
// Public API
// ──────────────────────────────────────────────────────────────────────────────

/**
 * Populate the registry with the full spec list from Rust.
 * Called at startup after `commands.listCommands()` resolves.
 */
export function populateRegistry(specs: CommandSpec[]): void {
  _registry.specs.clear();
  for (const spec of specs) {
    _registry.specs.set(spec.id, spec);
  }
}

/**
 * Register a TypeScript-side handler for a command.
 * When registered, this handler takes priority over routing to Rust.
 * Use this for UI-only commands (view toggles, panel focus, etc.).
 */
export function registerCommandHandler(id: string, handler: CommandHandler): void {
  _registry.handlers.set(id, handler);
}

/**
 * The single execution entry point for all commands.
 * All command invocations in the frontend route through here.
 *
 * - If a TypeScript handler is registered, it is called directly.
 * - Otherwise the command is forwarded to Rust via `commands.runCommand`.
 * - Throws if the command id is not found in the spec registry.
 */
export async function dispatchCommand(id: string, args?: unknown): Promise<void> {
  const spec = _registry.specs.get(id);
  if (!spec) {
    throw new Error(`Unknown command: ${id}`);
  }

  const handler = _registry.handlers.get(id);
  if (handler) {
    // TypeScript-side handler (view toggles, UI commands, etc.)
    await handler(args);
    return;
  }

  // Route to Rust via tauri.
  // For commands that return data, callers that need the return value
  // should use commands.runCommand directly; this path fires and discards.
  await commands.runCommand(id, args ?? null);
}

/**
 * Return the CommandSpec for a given id, or undefined if not found.
 */
export function getSpec(id: string): CommandSpec | undefined {
  return _registry.specs.get(id);
}

/**
 * Return all registered CommandSpecs.
 */
export function listSpecs(): CommandSpec[] {
  return Array.from(_registry.specs.values());
}

// ──────────────────────────────────────────────────────────────────────────────
// Test helpers (not part of the public production API)
// ──────────────────────────────────────────────────────────────────────────────

/**
 * Clear all registry state. Intended for use in test `beforeEach` only.
 * @internal
 */
export function _resetRegistryForTesting(): void {
  _registry.specs.clear();
  _registry.handlers.clear();
}
