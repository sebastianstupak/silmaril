import { describe, it, expect, beforeEach } from 'vitest';
import { registerCommand, listFrontendCommands, clearFrontendCommands } from './registry';

beforeEach(() => clearFrontendCommands());

describe('registerCommand', () => {
  it('registers a command and returns it via list', () => {
    registerCommand({ id: 'ui.test', label: 'Test', category: 'Test', run: () => {} });
    const cmds = listFrontendCommands();
    expect(cmds).toHaveLength(1);
    expect(cmds[0].id).toBe('ui.test');
  });

  it('overwrites a command with the same id (TS-first deduplication)', () => {
    registerCommand({ id: 'ui.test', label: 'First', category: 'Test', run: () => {} });
    registerCommand({ id: 'ui.test', label: 'Second', category: 'Test', run: () => {} });
    const cmds = listFrontendCommands();
    expect(cmds).toHaveLength(1);
    expect(cmds[0].label).toBe('Second');
  });

  it('stores multiple distinct commands', () => {
    registerCommand({ id: 'ui.a', label: 'A', category: 'Test', run: () => {} });
    registerCommand({ id: 'ui.b', label: 'B', category: 'Test', run: () => {} });
    expect(listFrontendCommands()).toHaveLength(2);
  });
});
