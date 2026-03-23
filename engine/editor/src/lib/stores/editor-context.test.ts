import { describe, it, expect, vi, beforeEach } from 'vitest';

vi.mock('$lib/template/state', () => ({
  getTemplateState: vi.fn(() => ({ entities: [], selectedEntityId: null })),
  getSelectedEntity: vi.fn(() => null),
  subscribeTemplate: vi.fn(() => () => {}),
}));
vi.mock('$lib/template/commands', () => ({
  selectEntity: vi.fn(),
  populateFromScan: vi.fn(),
}));

describe('editor-context — getSelectedEntityId', () => {
  it('returns null when nothing is selected', async () => {
    const { getSelectedEntityId } = await import('$lib/stores/editor-context');
    expect(getSelectedEntityId()).toBeNull();
  });
});
