import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

// ──────────────────────────────────────────────────────────────────────────────
// Helpers
// ──────────────────────────────────────────────────────────────────────────────

/** Mock factories — reset per describe block. */
let mockRunCommand: ReturnType<typeof vi.fn>;
let mockLogInfo: ReturnType<typeof vi.fn>;
let mockLogWarn: ReturnType<typeof vi.fn>;
let mockLogError: ReturnType<typeof vi.fn>;

/** Re-import the store fresh after mocks are wired (avoids singleton pollution). */
async function loadStore() {
  const mod = await import('$lib/stores/undo-history');
  return mod;
}

/** Wrap a value in the Result<ok> shape that commands.runCommand returns. */
function ok(data: unknown) {
  return { status: 'ok' as const, data };
}

function setupMocks() {
  mockRunCommand = vi.fn().mockResolvedValue(ok(null));
  mockLogInfo = vi.fn();
  mockLogWarn = vi.fn();
  mockLogError = vi.fn();

  vi.doMock('$lib/bindings', () => ({
    commands: {
      runCommand: mockRunCommand,
      listCommands: vi.fn().mockResolvedValue([]),
    },
  }));

  vi.doMock('$lib/api', () => ({
    sceneUndo: vi.fn().mockResolvedValue({ canUndo: false, canRedo: false }),
    sceneRedo: vi.fn().mockResolvedValue({ canUndo: false, canRedo: false }),
  }));

  vi.doMock('$lib/stores/console', () => ({
    logInfo: mockLogInfo,
    logWarn: mockLogWarn,
    logError: mockLogError,
  }));
}

// ──────────────────────────────────────────────────────────────────────────────
// Tests: setActiveTemplate
// ──────────────────────────────────────────────────────────────────────────────

describe('undo-history — setActiveTemplate', () => {
  beforeEach(() => {
    vi.resetModules();
    setupMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('setActiveTemplate(null) sets canUndo=false and canRedo=false', async () => {
    const store = await loadStore();
    await store.setActiveTemplate(null);
    expect(store.getCanUndo()).toBe(false);
    expect(store.getCanRedo()).toBe(false);
  });

  it('setActiveTemplate(null) clears the active path', async () => {
    const store = await loadStore();
    await store.setActiveTemplate(null);
    expect(store.getActiveTemplatePath()).toBeNull();
  });

  it('setActiveTemplate(path) stores the path', async () => {
    const store = await loadStore();
    mockRunCommand.mockResolvedValue(ok([]));
    await store.setActiveTemplate('/tmp/hero.yaml');
    expect(store.getActiveTemplatePath()).toBe('/tmp/hero.yaml');
  });

  it('setActiveTemplate(path) calls template.history to refresh state', async () => {
    const store = await loadStore();
    mockRunCommand.mockResolvedValue(ok([]));
    await store.setActiveTemplate('/tmp/hero.yaml');
    expect(mockRunCommand).toHaveBeenCalledWith('template.history', { template_path: '/tmp/hero.yaml' });
  });

  it('setActiveTemplate with non-empty history → canUndo=true', async () => {
    const store = await loadStore();
    mockRunCommand.mockResolvedValue(ok([
      { action_id: 1, description: 'Create entity' },
    ]));
    await store.setActiveTemplate('/tmp/hero.yaml');
    expect(store.getCanUndo()).toBe(true);
  });

  it('setActiveTemplate with empty history → canUndo=false', async () => {
    const store = await loadStore();
    mockRunCommand.mockResolvedValue(ok([]));
    await store.setActiveTemplate('/tmp/hero.yaml');
    expect(store.getCanUndo()).toBe(false);
  });

  it('setActiveTemplate(null) notifies subscribers', async () => {
    const store = await loadStore();
    const listener = vi.fn();
    store.subscribeUndoHistory(listener);
    await store.setActiveTemplate(null);
    expect(listener).toHaveBeenCalled();
  });

  it('setActiveTemplate(path) notifies subscribers after history fetch', async () => {
    const store = await loadStore();
    mockRunCommand.mockResolvedValue(ok([]));
    const listener = vi.fn();
    store.subscribeUndoHistory(listener);
    await store.setActiveTemplate('/tmp/hero.yaml');
    expect(listener).toHaveBeenCalled();
  });

  it('swallows template.history errors silently (template not yet open)', async () => {
    const store = await loadStore();
    mockRunCommand.mockRejectedValue(new Error('not open'));
    // Should not throw
    await expect(store.setActiveTemplate('/tmp/hero.yaml')).resolves.toBeUndefined();
  });
});

// ──────────────────────────────────────────────────────────────────────────────
// Tests: undo
// ──────────────────────────────────────────────────────────────────────────────

describe('undo-history — undo()', () => {
  beforeEach(() => {
    vi.resetModules();
    setupMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('undo with no template open logs a warning', async () => {
    const store = await loadStore();
    await store.undo();
    expect(mockLogWarn).toHaveBeenCalledWith(expect.stringContaining('no template'));
    expect(mockRunCommand).not.toHaveBeenCalledWith('template.undo', expect.anything());
  });

  it('undo calls template.undo with the active path', async () => {
    const store = await loadStore();
    // First call (setActiveTemplate → _refreshState): history = []
    mockRunCommand.mockResolvedValueOnce(ok([]));
    await store.setActiveTemplate('/tmp/hero.yaml');
    // Second call (undo): returns action id 42
    mockRunCommand.mockResolvedValueOnce(ok(42));
    // Third call (undo → _refreshState): returns []
    mockRunCommand.mockResolvedValueOnce(ok([]));
    await store.undo();
    expect(mockRunCommand).toHaveBeenCalledWith('template.undo', { template_path: '/tmp/hero.yaml' });
  });

  it('undo with actionId returned → logs info with action id', async () => {
    const store = await loadStore();
    mockRunCommand.mockResolvedValueOnce(ok([{ action_id: 1, description: 'x' }]));
    await store.setActiveTemplate('/tmp/hero.yaml');
    mockRunCommand.mockResolvedValueOnce(ok(7));
    mockRunCommand.mockResolvedValueOnce(ok([]));
    await store.undo();
    expect(mockLogInfo).toHaveBeenCalledWith(expect.stringContaining('7'));
  });

  it('undo with actionId returned → sets canRedo=true', async () => {
    const store = await loadStore();
    mockRunCommand.mockResolvedValueOnce(ok([{ action_id: 1, description: 'x' }]));
    await store.setActiveTemplate('/tmp/hero.yaml');
    mockRunCommand.mockResolvedValueOnce(ok(1));
    mockRunCommand.mockResolvedValueOnce(ok([]));
    await store.undo();
    expect(store.getCanRedo()).toBe(true);
  });

  it('undo with null result (nothing to undo) → logs "Nothing to undo"', async () => {
    const store = await loadStore();
    mockRunCommand.mockResolvedValueOnce(ok([]));
    await store.setActiveTemplate('/tmp/hero.yaml');
    mockRunCommand.mockResolvedValueOnce(ok(null));
    await store.undo();
    expect(mockLogInfo).toHaveBeenCalledWith(expect.stringContaining('Nothing to undo'));
  });

  it('undo refreshes state (calls template.history again)', async () => {
    const store = await loadStore();
    mockRunCommand.mockResolvedValueOnce(ok([]));
    await store.setActiveTemplate('/tmp/hero.yaml');
    mockRunCommand.mockResolvedValueOnce(ok(1));
    mockRunCommand.mockResolvedValueOnce(ok([]));
    await store.undo();
    // Once during setActiveTemplate (template.history), once after undo (template.history)
    const historyCalls = mockRunCommand.mock.calls.filter(
      (call) => call[0] === 'template.history'
    );
    expect(historyCalls).toHaveLength(2);
  });

  it('undo error → logs error message', async () => {
    const store = await loadStore();
    mockRunCommand.mockResolvedValueOnce(ok([]));
    await store.setActiveTemplate('/tmp/hero.yaml');
    mockRunCommand.mockRejectedValueOnce(new Error('disk full'));
    await store.undo();
    expect(mockLogError).toHaveBeenCalledWith(expect.stringContaining('Undo failed'));
  });
});

// ──────────────────────────────────────────────────────────────────────────────
// Tests: redo
// ──────────────────────────────────────────────────────────────────────────────

describe('undo-history — redo()', () => {
  beforeEach(() => {
    vi.resetModules();
    setupMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('redo with no template open logs a warning', async () => {
    const store = await loadStore();
    await store.redo();
    expect(mockLogWarn).toHaveBeenCalledWith(expect.stringContaining('no template'));
    expect(mockRunCommand).not.toHaveBeenCalledWith('template.redo', expect.anything());
  });

  it('redo calls template.redo with the active path', async () => {
    const store = await loadStore();
    mockRunCommand.mockResolvedValueOnce(ok([]));
    await store.setActiveTemplate('/tmp/hero.yaml');
    mockRunCommand.mockResolvedValueOnce(ok(42));
    mockRunCommand.mockResolvedValueOnce(ok([]));
    await store.redo();
    expect(mockRunCommand).toHaveBeenCalledWith('template.redo', { template_path: '/tmp/hero.yaml' });
  });

  it('redo with actionId returned → logs info with action id', async () => {
    const store = await loadStore();
    mockRunCommand.mockResolvedValueOnce(ok([]));
    await store.setActiveTemplate('/tmp/hero.yaml');
    mockRunCommand.mockResolvedValueOnce(ok(3));
    mockRunCommand.mockResolvedValueOnce(ok([]));
    await store.redo();
    expect(mockLogInfo).toHaveBeenCalledWith(expect.stringContaining('3'));
  });

  it('redo with null result (nothing to redo) → logs "Nothing to redo"', async () => {
    const store = await loadStore();
    mockRunCommand.mockResolvedValueOnce(ok([]));
    await store.setActiveTemplate('/tmp/hero.yaml');
    mockRunCommand.mockResolvedValueOnce(ok(null));
    await store.redo();
    expect(mockLogInfo).toHaveBeenCalledWith(expect.stringContaining('Nothing to redo'));
  });

  it('redo with null result → canRedo becomes false', async () => {
    const store = await loadStore();
    mockRunCommand.mockResolvedValueOnce(ok([]));
    await store.setActiveTemplate('/tmp/hero.yaml');
    mockRunCommand.mockResolvedValueOnce(ok(null));
    await store.redo();
    expect(store.getCanRedo()).toBe(false);
  });

  it('redo refreshes state (calls template.history again)', async () => {
    const store = await loadStore();
    mockRunCommand.mockResolvedValueOnce(ok([]));
    await store.setActiveTemplate('/tmp/hero.yaml');
    mockRunCommand.mockResolvedValueOnce(ok(1));
    mockRunCommand.mockResolvedValueOnce(ok([]));
    await store.redo();
    const historyCalls = mockRunCommand.mock.calls.filter(
      (call) => call[0] === 'template.history'
    );
    expect(historyCalls).toHaveLength(2);
  });

  it('redo error → logs error message', async () => {
    const store = await loadStore();
    mockRunCommand.mockResolvedValueOnce(ok([]));
    await store.setActiveTemplate('/tmp/hero.yaml');
    mockRunCommand.mockRejectedValueOnce(new Error('io error'));
    await store.redo();
    expect(mockLogError).toHaveBeenCalledWith(expect.stringContaining('Redo failed'));
  });
});

// ──────────────────────────────────────────────────────────────────────────────
// Tests: onTemplateMutated
// ──────────────────────────────────────────────────────────────────────────────

describe('undo-history — onTemplateMutated()', () => {
  beforeEach(() => {
    vi.resetModules();
    setupMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('onTemplateMutated resets canRedo to false', async () => {
    const store = await loadStore();
    // Manually get canRedo to true by simulating undo
    mockRunCommand.mockResolvedValueOnce(ok([{ action_id: 1, description: 'x' }]));
    await store.setActiveTemplate('/tmp/hero.yaml');
    mockRunCommand.mockResolvedValueOnce(ok(1));
    mockRunCommand.mockResolvedValueOnce(ok([]));
    await store.undo(); // canRedo becomes true
    // Now mutate — should clear canRedo
    mockRunCommand.mockResolvedValueOnce(ok([{ action_id: 2, description: 'y' }]));
    await store.onTemplateMutated();
    expect(store.getCanRedo()).toBe(false);
  });

  it('onTemplateMutated refreshes canUndo from history', async () => {
    const store = await loadStore();
    mockRunCommand.mockResolvedValueOnce(ok([]));
    await store.setActiveTemplate('/tmp/hero.yaml');
    // Now simulate a mutation that added to history
    mockRunCommand.mockResolvedValueOnce(ok([{ action_id: 1, description: 'add entity' }]));
    await store.onTemplateMutated();
    expect(store.getCanUndo()).toBe(true);
  });

  it('onTemplateMutated notifies subscribers', async () => {
    const store = await loadStore();
    mockRunCommand.mockResolvedValueOnce(ok([]));
    await store.setActiveTemplate('/tmp/hero.yaml');
    const listener = vi.fn();
    store.subscribeUndoHistory(listener);
    listener.mockClear();
    mockRunCommand.mockResolvedValueOnce(ok([]));
    await store.onTemplateMutated();
    expect(listener).toHaveBeenCalled();
  });
});

// ──────────────────────────────────────────────────────────────────────────────
// Tests: subscribeUndoHistory
// ──────────────────────────────────────────────────────────────────────────────

describe('undo-history — subscribeUndoHistory()', () => {
  beforeEach(() => {
    vi.resetModules();
    setupMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('listener is called when state changes', async () => {
    const store = await loadStore();
    const listener = vi.fn();
    store.subscribeUndoHistory(listener);
    await store.setActiveTemplate(null);
    expect(listener).toHaveBeenCalled();
  });

  it('unsubscribe stops listener from being called', async () => {
    const store = await loadStore();
    const listener = vi.fn();
    const unsub = store.subscribeUndoHistory(listener);
    unsub();
    listener.mockClear();
    await store.setActiveTemplate(null);
    expect(listener).not.toHaveBeenCalled();
  });

  it('multiple listeners all receive notification', async () => {
    const store = await loadStore();
    const l1 = vi.fn();
    const l2 = vi.fn();
    store.subscribeUndoHistory(l1);
    store.subscribeUndoHistory(l2);
    await store.setActiveTemplate(null);
    expect(l1).toHaveBeenCalled();
    expect(l2).toHaveBeenCalled();
  });

  it('only unsubscribed listener stops — others continue', async () => {
    const store = await loadStore();
    const l1 = vi.fn();
    const l2 = vi.fn();
    const unsub1 = store.subscribeUndoHistory(l1);
    store.subscribeUndoHistory(l2);
    unsub1();
    l1.mockClear(); l2.mockClear();
    await store.setActiveTemplate(null);
    expect(l1).not.toHaveBeenCalled();
    expect(l2).toHaveBeenCalled();
  });
});
