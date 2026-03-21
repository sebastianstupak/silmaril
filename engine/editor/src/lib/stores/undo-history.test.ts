import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

// ──────────────────────────────────────────────────────────────────────────────
// Helpers
// ──────────────────────────────────────────────────────────────────────────────

/** Mock factories — reset per describe block. */
let mockTemplateHistory: ReturnType<typeof vi.fn>;
let mockTemplateUndo: ReturnType<typeof vi.fn>;
let mockTemplateRedo: ReturnType<typeof vi.fn>;
let mockLogInfo: ReturnType<typeof vi.fn>;
let mockLogWarn: ReturnType<typeof vi.fn>;
let mockLogError: ReturnType<typeof vi.fn>;

/** Re-import the store fresh after mocks are wired (avoids singleton pollution). */
async function loadStore() {
  const mod = await import('$lib/stores/undo-history');
  return mod;
}

function setupMocks() {
  mockTemplateHistory = vi.fn().mockResolvedValue([]);
  mockTemplateUndo = vi.fn().mockResolvedValue(null);
  mockTemplateRedo = vi.fn().mockResolvedValue(null);
  mockLogInfo = vi.fn();
  mockLogWarn = vi.fn();
  mockLogError = vi.fn();

  vi.doMock('$lib/api', () => ({
    templateHistory: mockTemplateHistory,
    templateUndo: mockTemplateUndo,
    templateRedo: mockTemplateRedo,
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
    mockTemplateHistory.mockResolvedValue([]);
    await store.setActiveTemplate('/tmp/hero.yaml');
    expect(store.getActiveTemplatePath()).toBe('/tmp/hero.yaml');
  });

  it('setActiveTemplate(path) calls templateHistory to refresh state', async () => {
    const store = await loadStore();
    mockTemplateHistory.mockResolvedValue([]);
    await store.setActiveTemplate('/tmp/hero.yaml');
    expect(mockTemplateHistory).toHaveBeenCalledWith('/tmp/hero.yaml');
  });

  it('setActiveTemplate with non-empty history → canUndo=true', async () => {
    const store = await loadStore();
    mockTemplateHistory.mockResolvedValue([
      { action_id: 1, description: 'Create entity' },
    ]);
    await store.setActiveTemplate('/tmp/hero.yaml');
    expect(store.getCanUndo()).toBe(true);
  });

  it('setActiveTemplate with empty history → canUndo=false', async () => {
    const store = await loadStore();
    mockTemplateHistory.mockResolvedValue([]);
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
    mockTemplateHistory.mockResolvedValue([]);
    const listener = vi.fn();
    store.subscribeUndoHistory(listener);
    await store.setActiveTemplate('/tmp/hero.yaml');
    expect(listener).toHaveBeenCalled();
  });

  it('swallows templateHistory errors silently (template not yet open)', async () => {
    const store = await loadStore();
    mockTemplateHistory.mockRejectedValue(new Error('not open'));
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
    expect(mockTemplateUndo).not.toHaveBeenCalled();
  });

  it('undo calls templateUndo with the active path', async () => {
    const store = await loadStore();
    mockTemplateHistory.mockResolvedValue([]);
    await store.setActiveTemplate('/tmp/hero.yaml');
    mockTemplateUndo.mockResolvedValue(42);
    mockTemplateHistory.mockResolvedValue([]);
    await store.undo();
    expect(mockTemplateUndo).toHaveBeenCalledWith('/tmp/hero.yaml');
  });

  it('undo with actionId returned → logs info with action id', async () => {
    const store = await loadStore();
    mockTemplateHistory.mockResolvedValue([{ action_id: 1, description: 'x' }]);
    await store.setActiveTemplate('/tmp/hero.yaml');
    mockTemplateUndo.mockResolvedValue(7);
    mockTemplateHistory.mockResolvedValue([]);
    await store.undo();
    expect(mockLogInfo).toHaveBeenCalledWith(expect.stringContaining('7'));
  });

  it('undo with actionId returned → sets canRedo=true', async () => {
    const store = await loadStore();
    mockTemplateHistory.mockResolvedValue([{ action_id: 1, description: 'x' }]);
    await store.setActiveTemplate('/tmp/hero.yaml');
    mockTemplateUndo.mockResolvedValue(1);
    mockTemplateHistory.mockResolvedValue([]);
    await store.undo();
    expect(store.getCanRedo()).toBe(true);
  });

  it('undo with null result (nothing to undo) → logs "Nothing to undo"', async () => {
    const store = await loadStore();
    mockTemplateHistory.mockResolvedValue([]);
    await store.setActiveTemplate('/tmp/hero.yaml');
    mockTemplateUndo.mockResolvedValue(null);
    await store.undo();
    expect(mockLogInfo).toHaveBeenCalledWith(expect.stringContaining('Nothing to undo'));
  });

  it('undo refreshes state (calls templateHistory again)', async () => {
    const store = await loadStore();
    mockTemplateHistory.mockResolvedValue([]);
    await store.setActiveTemplate('/tmp/hero.yaml');
    mockTemplateUndo.mockResolvedValue(1);
    mockTemplateHistory.mockResolvedValue([]);
    await store.undo();
    // Once during setActiveTemplate, once after undo
    expect(mockTemplateHistory).toHaveBeenCalledTimes(2);
  });

  it('undo error → logs error message', async () => {
    const store = await loadStore();
    mockTemplateHistory.mockResolvedValue([]);
    await store.setActiveTemplate('/tmp/hero.yaml');
    mockTemplateUndo.mockRejectedValue(new Error('disk full'));
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
    expect(mockTemplateRedo).not.toHaveBeenCalled();
  });

  it('redo calls templateRedo with the active path', async () => {
    const store = await loadStore();
    mockTemplateHistory.mockResolvedValue([]);
    await store.setActiveTemplate('/tmp/hero.yaml');
    mockTemplateRedo.mockResolvedValue(42);
    mockTemplateHistory.mockResolvedValue([]);
    await store.redo();
    expect(mockTemplateRedo).toHaveBeenCalledWith('/tmp/hero.yaml');
  });

  it('redo with actionId returned → logs info with action id', async () => {
    const store = await loadStore();
    mockTemplateHistory.mockResolvedValue([]);
    await store.setActiveTemplate('/tmp/hero.yaml');
    mockTemplateRedo.mockResolvedValue(3);
    mockTemplateHistory.mockResolvedValue([]);
    await store.redo();
    expect(mockLogInfo).toHaveBeenCalledWith(expect.stringContaining('3'));
  });

  it('redo with null result (nothing to redo) → logs "Nothing to redo"', async () => {
    const store = await loadStore();
    mockTemplateHistory.mockResolvedValue([]);
    await store.setActiveTemplate('/tmp/hero.yaml');
    mockTemplateRedo.mockResolvedValue(null);
    await store.redo();
    expect(mockLogInfo).toHaveBeenCalledWith(expect.stringContaining('Nothing to redo'));
  });

  it('redo with null result → canRedo becomes false', async () => {
    const store = await loadStore();
    mockTemplateHistory.mockResolvedValue([]);
    await store.setActiveTemplate('/tmp/hero.yaml');
    mockTemplateRedo.mockResolvedValue(null);
    await store.redo();
    expect(store.getCanRedo()).toBe(false);
  });

  it('redo refreshes state (calls templateHistory again)', async () => {
    const store = await loadStore();
    mockTemplateHistory.mockResolvedValue([]);
    await store.setActiveTemplate('/tmp/hero.yaml');
    mockTemplateRedo.mockResolvedValue(1);
    mockTemplateHistory.mockResolvedValue([]);
    await store.redo();
    expect(mockTemplateHistory).toHaveBeenCalledTimes(2);
  });

  it('redo error → logs error message', async () => {
    const store = await loadStore();
    mockTemplateHistory.mockResolvedValue([]);
    await store.setActiveTemplate('/tmp/hero.yaml');
    mockTemplateRedo.mockRejectedValue(new Error('io error'));
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
    mockTemplateHistory.mockResolvedValue([{ action_id: 1, description: 'x' }]);
    await store.setActiveTemplate('/tmp/hero.yaml');
    mockTemplateUndo.mockResolvedValue(1);
    mockTemplateHistory.mockResolvedValue([]);
    await store.undo(); // canRedo becomes true
    // Now mutate — should clear canRedo
    mockTemplateHistory.mockResolvedValue([{ action_id: 2, description: 'y' }]);
    await store.onTemplateMutated();
    expect(store.getCanRedo()).toBe(false);
  });

  it('onTemplateMutated refreshes canUndo from history', async () => {
    const store = await loadStore();
    mockTemplateHistory.mockResolvedValue([]);
    await store.setActiveTemplate('/tmp/hero.yaml');
    // Now simulate a mutation that added to history
    mockTemplateHistory.mockResolvedValue([{ action_id: 1, description: 'add entity' }]);
    await store.onTemplateMutated();
    expect(store.getCanUndo()).toBe(true);
  });

  it('onTemplateMutated notifies subscribers', async () => {
    const store = await loadStore();
    mockTemplateHistory.mockResolvedValue([]);
    await store.setActiveTemplate('/tmp/hero.yaml');
    const listener = vi.fn();
    store.subscribeUndoHistory(listener);
    listener.mockClear();
    mockTemplateHistory.mockResolvedValue([]);
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
