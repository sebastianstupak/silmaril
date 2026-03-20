import type { FrontendCommand } from './types';

const _commands = new Map<string, FrontendCommand>();

export function registerCommand(cmd: FrontendCommand): void {
  _commands.set(cmd.id, cmd);
}

export function listFrontendCommands(): FrontendCommand[] {
  return Array.from(_commands.values());
}

/** Test-only: clear all registered commands. */
export function clearFrontendCommands(): void {
  _commands.clear();
}
