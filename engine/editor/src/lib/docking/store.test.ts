import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest';
import type { EditorLayout, TabsNode, SplitNode } from './types';
import {
  removePanelFromLayout,
  dropPanel,
  setActiveTab,
  resizeSplit,
  loadSavedLayouts,
  saveSavedLayouts,
  initialSavedLayouts,
  defaultLayout,
} from './store';

function makeLayoutWithSplitBottom(): EditorLayout {
  return {
    root: { type: 'tabs', activeTab: 0, panels: ['hierarchy'] },
    bottomPanel: {
      type: 'split',
      direction: 'horizontal',
      sizes: [50, 50],
      children: [
        { type: 'tabs', activeTab: 0, panels: ['console'] },
        { type: 'tabs', activeTab: 0, panels: ['profiler'] },
      ],
    },
  };
}

function makeLayoutWithFlatBottom(): EditorLayout {
  return {
    root: { type: 'tabs', activeTab: 0, panels: ['hierarchy'] },
    bottomPanel: { type: 'tabs', activeTab: 0, panels: ['console', 'profiler'] },
  };
}

describe('removePanelFromLayout', () => {
  it('removes panel from flat bottomPanel', () => {
    const layout = makeLayoutWithFlatBottom();
    const result = removePanelFromLayout(layout, 'profiler');
    const bp = result.bottomPanel as TabsNode;
    expect(bp.panels).toEqual(['console']);
  });

  it('removes panel from split bottomPanel', () => {
    const layout = makeLayoutWithSplitBottom();
    const result = removePanelFromLayout(layout, 'profiler');
    expect(result.bottomPanel.type).toBe('tabs');
    const bp = result.bottomPanel as TabsNode;
    expect(bp.panels).toEqual(['console']);
  });

  it('removes panel from root, leaves bottomPanel untouched', () => {
    const layout = makeLayoutWithFlatBottom();
    const result = removePanelFromLayout(layout, 'hierarchy');
    expect((result.root as TabsNode).panels).toEqual([]);
    const bp = result.bottomPanel as TabsNode;
    expect(bp.panels).toEqual(['console', 'profiler']);
  });

  it('leaves empty TabsNode when all panels removed from split bottomPanel', () => {
    const layout = makeLayoutWithSplitBottom();
    // Remove console first
    const after1 = removePanelFromLayout(layout, 'console');
    // Remove profiler — bottomPanel should now be an empty TabsNode
    const after2 = removePanelFromLayout(after1, 'profiler');
    expect(after2.bottomPanel.type).toBe('tabs');
    expect((after2.bottomPanel as TabsNode).panels).toEqual([]);
  });
});

describe('dropPanel — isBottomPanel=true', () => {
  it('center drop adds panel as tab in flat bottomPanel', () => {
    const layout = makeLayoutWithFlatBottom();
    const result = dropPanel(layout, 'hierarchy', [], 'center', true);
    const bp = result.bottomPanel as TabsNode;
    expect(bp.panels).toContain('hierarchy');
    expect((result.root as TabsNode).panels).not.toContain('hierarchy');
  });

  it('right drop creates horizontal split in bottomPanel', () => {
    const layout = makeLayoutWithFlatBottom();
    const result = dropPanel(layout, 'hierarchy', [], 'right', true);
    expect(result.bottomPanel.type).toBe('split');
    expect((result.bottomPanel as SplitNode).direction).toBe('horizontal');
    expect(JSON.stringify(result.bottomPanel)).toContain('hierarchy');
  });

  it('left drop creates horizontal split with new panel on left', () => {
    const layout = makeLayoutWithFlatBottom();
    const result = dropPanel(layout, 'hierarchy', [], 'left', true);
    expect(result.bottomPanel.type).toBe('split');
    const bp = result.bottomPanel as SplitNode;
    expect(bp.direction).toBe('horizontal');
    const firstChild = bp.children[0];
    expect(firstChild.type).toBe('tabs');
    expect((firstChild as TabsNode).panels).toContain('hierarchy');
  });

  it('panel is not lost after drop — removed from source, appears in target', () => {
    const layout = makeLayoutWithFlatBottom();
    const result = dropPanel(layout, 'hierarchy', [], 'right', true);
    expect((result.root as TabsNode).panels).not.toContain('hierarchy');
    expect(JSON.stringify(result.bottomPanel)).toContain('hierarchy');
  });

  it('center drop on split bottomPanel adds panel to first TabsNode', () => {
    const layout = makeLayoutWithSplitBottom();
    // bottomPanel is a split with console (left) and profiler (right)
    const result = dropPanel(layout, 'hierarchy', [], 'center', true);
    // hierarchy should be added as a tab, not create another split level
    expect(result.bottomPanel.type).toBe('split');
    const firstChild = (result.bottomPanel as SplitNode).children[0];
    expect(firstChild.type).toBe('tabs');
    expect((firstChild as TabsNode).panels).toContain('hierarchy');
  });
});

describe('setActiveTab — isBottomPanel=true', () => {
  it('sets activeTab on flat bottomPanel', () => {
    const layout = makeLayoutWithFlatBottom();
    const result = setActiveTab(layout, [], 1, true);
    expect((result.bottomPanel as TabsNode).activeTab).toBe(1);
  });

  it('sets activeTab on nested TabsNode via path in split bottomPanel', () => {
    const layout = makeLayoutWithSplitBottom();
    const result = setActiveTab(layout, [1], 0, true);
    if (result.bottomPanel.type === 'split') {
      const child = result.bottomPanel.children[1];
      expect(child.type).toBe('tabs');
      if (child.type === 'tabs') expect(child.activeTab).toBe(0);
    }
  });
});

describe('resizeSplit — isBottomPanel=true', () => {
  it('resizes a split node inside bottomPanel', () => {
    const layout = makeLayoutWithSplitBottom();
    const result = resizeSplit(layout, [], 1, 50, 1000, true);
    expect(result.bottomPanel.type).toBe('split');
    const bp = result.bottomPanel as SplitNode;
    expect(bp.sizes[0]).toBeCloseTo(55, 0);
    expect(bp.sizes[1]).toBeCloseTo(45, 0);
  });
});

// ── removePanelFromLayout (root tree) ─────────────────────────────────────────

function threeColumn(): EditorLayout {
  return {
    root: {
      type: 'split', direction: 'horizontal', sizes: [20, 55, 25],
      children: [
        { type: 'tabs', activeTab: 0, panels: ['hierarchy'] },
        { type: 'tabs', activeTab: 0, panels: ['viewport'] },
        { type: 'tabs', activeTab: 0, panels: ['inspector'] },
      ],
    },
    bottomPanel: { type: 'tabs', activeTab: 0, panels: ['console'] },
  };
}

describe('removePanelFromLayout — root tree', () => {
  it('does not mutate the original layout', () => {
    const original = threeColumn();
    removePanelFromLayout(original, 'viewport');
    expect((original.root as SplitNode).children[1]).toMatchObject({ panels: ['viewport'] });
  });

  it('collapses a split when a child tabs node becomes empty', () => {
    const result = removePanelFromLayout(threeColumn(), 'viewport');
    const root = result.root as SplitNode;
    expect(root.children).toHaveLength(2);
  });

  it('re-normalises split sizes to sum to 100 after removal', () => {
    const result = removePanelFromLayout(threeColumn(), 'viewport');
    const root = result.root as SplitNode;
    const total = root.sizes.reduce((a, b) => a + b, 0);
    expect(total).toBeCloseTo(100, 5);
  });

  it('adjusts activeTab when the last tab is removed', () => {
    const layout: EditorLayout = {
      root: { type: 'tabs', activeTab: 2, panels: ['a', 'b', 'c'] },
      bottomPanel: { type: 'tabs', activeTab: 0, panels: [] },
    };
    const result = removePanelFromLayout(layout, 'c');
    expect((result.root as TabsNode).activeTab).toBe(1);
  });

  it('is a no-op when the panel does not exist — same panels, same structure', () => {
    const original = threeColumn();
    const result = removePanelFromLayout(original, 'nonexistent');
    // cleanNode re-normalises sizes even on a no-op (floating-point safe comparison)
    const collectPanels = (n: any): string[] =>
      n.type === 'tabs' ? n.panels : n.children.flatMap(collectPanels);
    expect(collectPanels(result.root)).toEqual(collectPanels(original.root));
    expect(result.bottomPanel).toMatchObject({ panels: ['console'] });
  });
});

// ── dropPanel (root tree) ─────────────────────────────────────────────────────

describe('dropPanel — root tree', () => {
  it('adds panel to an existing tabs node on center drop', () => {
    const result = dropPanel(threeColumn(), 'assets', [0], 'center', false);
    const col0 = (result.root as SplitNode).children[0] as TabsNode;
    expect(col0.panels).toContain('assets');
  });

  it('does not duplicate the panel — removes from source first', () => {
    const result = dropPanel(threeColumn(), 'hierarchy', [1], 'center', false);
    const allPanels: string[] = [];
    function collect(n: any) {
      if (n.type === 'tabs') allPanels.push(...n.panels);
      else n.children.forEach(collect);
    }
    collect(result.root);
    expect(allPanels.filter(p => p === 'hierarchy')).toHaveLength(1);
  });

  it('left drop creates a horizontal split with new panel first', () => {
    const layout: EditorLayout = {
      root: { type: 'tabs', activeTab: 0, panels: ['viewport'] },
      bottomPanel: { type: 'tabs', activeTab: 0, panels: [] },
    };
    const result = dropPanel(layout, 'inspector', [], 'left', false);
    const root = result.root as SplitNode;
    expect(root.direction).toBe('horizontal');
    expect((root.children[0] as TabsNode).panels).toContain('inspector');
  });

  it('right drop creates a horizontal split with new panel last', () => {
    const layout: EditorLayout = {
      root: { type: 'tabs', activeTab: 0, panels: ['viewport'] },
      bottomPanel: { type: 'tabs', activeTab: 0, panels: [] },
    };
    const result = dropPanel(layout, 'inspector', [], 'right', false);
    const root = result.root as SplitNode;
    expect((root.children[1] as TabsNode).panels).toContain('inspector');
  });

  it('does not mutate the original layout', () => {
    const original = threeColumn();
    dropPanel(original, 'assets', [0], 'center', false);
    expect((original.root as SplitNode).children[0]).toMatchObject({ panels: ['hierarchy'] });
  });
});

// ── resizeSplit (root tree) ───────────────────────────────────────────────────

describe('resizeSplit — root tree', () => {
  it('grows the left child and shrinks the right on positive delta', () => {
    const result = resizeSplit(threeColumn(), [], 1, 100, 1000, false);
    const root = result.root as SplitNode;
    expect(root.sizes[0]).toBeGreaterThan(20);
    expect(root.sizes[1]).toBeLessThan(55);
  });

  it('clamps both children to a minimum of 5%', () => {
    const result = resizeSplit(threeColumn(), [], 1, -10000, 1000, false);
    const root = result.root as SplitNode;
    expect(root.sizes[0]).toBeGreaterThanOrEqual(5);
    expect(root.sizes[1]).toBeGreaterThanOrEqual(5);
  });

  it('keeps sizes summing to 100 after resize', () => {
    const result = resizeSplit(threeColumn(), [], 1, 50, 1000, false);
    const root = result.root as SplitNode;
    const total = root.sizes.reduce((a, b) => a + b, 0);
    expect(total).toBeCloseTo(100, 5);
  });

  it('returns layout unchanged for invalid divider index', () => {
    const original = threeColumn();
    const result = resizeSplit(original, [], 99, 50, 1000, false);
    expect(JSON.stringify(result)).toEqual(JSON.stringify(original));
  });
});

// ── loadSavedLayouts / saveSavedLayouts ───────────────────────────────────────

describe('loadSavedLayouts', () => {
  beforeEach(() => { localStorage.clear(); vi.useFakeTimers(); });
  afterEach(() => { vi.useRealTimers(); });

  it('returns three initial slots on a fresh install', () => {
    const layouts = loadSavedLayouts();
    expect(layouts).toHaveLength(3);
    expect(layouts.map(l => l.name)).toEqual(['Edit', 'Assets', 'Review']);
  });

  it('initial slots carry ctrl+1/2/3 keybinds', () => {
    const layouts = loadSavedLayouts();
    expect(layouts.map(l => l.keybind)).toEqual(['ctrl+1', 'ctrl+2', 'ctrl+3']);
  });

  it('returns deep clones — not the same object references', () => {
    const layouts = loadSavedLayouts();
    expect(layouts).not.toBe(initialSavedLayouts);
    expect(layouts[0]).not.toBe(initialSavedLayouts[0]);
  });

  it('returns stored layouts when valid data is present', () => {
    const stored = [{ id: 'x', name: 'Custom', layout: defaultLayout }];
    localStorage.setItem('silmaril-saved-layouts', JSON.stringify(stored));
    const layouts = loadSavedLayouts();
    expect(layouts[0].name).toBe('Custom');
  });

  it('falls back to initial slots on corrupt JSON', () => {
    localStorage.setItem('silmaril-saved-layouts', '{corrupt}');
    expect(loadSavedLayouts()).toHaveLength(3);
  });

  it('falls back to initial slots when stored array is empty', () => {
    localStorage.setItem('silmaril-saved-layouts', '[]');
    expect(loadSavedLayouts()).toHaveLength(3);
  });
});

describe('saveSavedLayouts', () => {
  beforeEach(() => { localStorage.clear(); vi.useFakeTimers(); });
  afterEach(() => { vi.useRealTimers(); });

  it('persists to localStorage after the debounce delay', () => {
    saveSavedLayouts([{ id: 'x', name: 'X', layout: defaultLayout }]);
    expect(localStorage.getItem('silmaril-saved-layouts')).toBeNull();
    vi.runAllTimers();
    const stored = JSON.parse(localStorage.getItem('silmaril-saved-layouts')!);
    expect(stored[0].name).toBe('X');
  });

  it('debounces — only the last call within the window is written', () => {
    saveSavedLayouts([{ id: 'a', name: 'First', layout: defaultLayout }]);
    saveSavedLayouts([{ id: 'b', name: 'Second', layout: defaultLayout }]);
    vi.runAllTimers();
    const stored = JSON.parse(localStorage.getItem('silmaril-saved-layouts')!);
    expect(stored[0].name).toBe('Second');
  });
});

// ── Layout lifecycle integration ──────────────────────────────────────────────

describe('layout lifecycle — integration', () => {
  beforeEach(() => { localStorage.clear(); vi.useFakeTimers(); });
  afterEach(() => { vi.useRealTimers(); });
  it('apply then move panel → JSON diff marks layout as dirty', () => {
    const slot = initialSavedLayouts[0];
    let current: EditorLayout = JSON.parse(JSON.stringify(slot.layout));
    expect(JSON.stringify(current)).toEqual(JSON.stringify(slot.layout)); // clean

    current = dropPanel(current, 'hierarchy', [1], 'center', false);
    expect(JSON.stringify(current)).not.toEqual(JSON.stringify(slot.layout)); // dirty
  });

  it('save-to-slot makes layout clean again', () => {
    const slot = initialSavedLayouts[0];
    let current: EditorLayout = JSON.parse(JSON.stringify(slot.layout));
    current = dropPanel(current, 'hierarchy', [1], 'center', false);

    const savedSlot = { ...slot, layout: JSON.parse(JSON.stringify(current)) };
    expect(JSON.stringify(savedSlot.layout)).toEqual(JSON.stringify(current));
  });

  it('reset-to-saved discards modifications', () => {
    const slot = initialSavedLayouts[0];
    let current: EditorLayout = JSON.parse(JSON.stringify(slot.layout));
    current = dropPanel(current, 'hierarchy', [1], 'center', false);

    current = JSON.parse(JSON.stringify(slot.layout)); // reset
    expect(JSON.stringify(current)).toEqual(JSON.stringify(slot.layout));
  });

  it('keybind ctrl+1 resolves to the Scene slot', () => {
    const slots = loadSavedLayouts();
    const matched = slots.find(s => s.keybind === 'ctrl+1');
    expect(matched?.name).toBe('Scene');
  });

  it('keybind follows the slot after rename (not position)', () => {
    let slots = loadSavedLayouts();
    slots = slots.map(s => s.id === 'builtin-scene' ? { ...s, name: 'Custom' } : s);
    const matched = slots.find(s => s.keybind === 'ctrl+1');
    expect(matched?.name).toBe('Custom');
  });

  it('duplicated slot has an independent layout copy', () => {
    const original = initialSavedLayouts[0];
    const copy = { ...original, id: 'copy', layout: JSON.parse(JSON.stringify(original.layout)) };
    copy.layout = dropPanel(copy.layout, 'hierarchy', [1], 'center', false);
    expect(JSON.stringify(copy.layout)).not.toEqual(JSON.stringify(original.layout));
  });

  it('deleting a slot removes it from the list', () => {
    let slots = loadSavedLayouts();
    slots = slots.filter(s => s.id !== 'builtin-scene');
    expect(slots).toHaveLength(2);
    expect(slots.find(s => s.id === 'builtin-scene')).toBeUndefined();
  });
});
