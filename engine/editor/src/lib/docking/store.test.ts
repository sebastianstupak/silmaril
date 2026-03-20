import { describe, it, expect } from 'vitest';
import type { EditorLayout, TabsNode } from './types';
import {
  removePanelFromLayout,
  dropPanel,
  setActiveTab,
  resizeSplit,
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
    if (result.bottomPanel.type === 'split') {
      expect(result.bottomPanel.direction).toBe('horizontal');
      const allPanels = JSON.stringify(result.bottomPanel);
      expect(allPanels).toContain('hierarchy');
    }
  });

  it('left drop creates horizontal split with new panel on left', () => {
    const layout = makeLayoutWithFlatBottom();
    const result = dropPanel(layout, 'hierarchy', [], 'left', true);
    expect(result.bottomPanel.type).toBe('split');
    if (result.bottomPanel.type === 'split') {
      expect(result.bottomPanel.direction).toBe('horizontal');
      const firstChild = result.bottomPanel.children[0];
      expect(firstChild.type).toBe('tabs');
      if (firstChild.type === 'tabs') {
        expect(firstChild.panels).toContain('hierarchy');
      }
    }
  });

  it('panel is not lost after drop — removed from source, appears in target', () => {
    const layout = makeLayoutWithFlatBottom();
    const result = dropPanel(layout, 'hierarchy', [], 'right', true);
    expect((result.root as TabsNode).panels).not.toContain('hierarchy');
    expect(JSON.stringify(result.bottomPanel)).toContain('hierarchy');
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
    if (result.bottomPanel.type === 'split') {
      expect(result.bottomPanel.sizes[0]).toBeCloseTo(55, 0);
      expect(result.bottomPanel.sizes[1]).toBeCloseTo(45, 0);
    }
  });
});
