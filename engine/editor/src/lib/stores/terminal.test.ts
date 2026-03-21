import { describe, it, expect, beforeEach } from 'vitest';
import {
  getTerminalState,
  addTab,
  closeTab,
  setActiveTab,
  markExited,
  appendTerminalData,
  drainTerminalData,
} from './terminal';

import { _resetForTest } from './terminal';

beforeEach(() => _resetForTest());

describe('addTab', () => {
  it('adds a tab and sets it as active', () => {
    addTab('tab-1');
    const s = getTerminalState();
    expect(s.tabs).toHaveLength(1);
    expect(s.tabs[0].id).toBe('tab-1');
    expect(s.tabs[0].title).toBe('Terminal 1');
    expect(s.tabs[0].exited).toBe(false);
    expect(s.activeTabId).toBe('tab-1');
  });

  it('increments title counter per call', () => {
    addTab('tab-1');
    addTab('tab-2');
    const s = getTerminalState();
    expect(s.tabs[0].title).toBe('Terminal 1');
    expect(s.tabs[1].title).toBe('Terminal 2');
  });

  it('new tab becomes active', () => {
    addTab('tab-1');
    addTab('tab-2');
    expect(getTerminalState().activeTabId).toBe('tab-2');
  });
});

describe('closeTab', () => {
  it('removes tab when multiple tabs exist', () => {
    addTab('tab-1');
    addTab('tab-2');
    closeTab('tab-1');
    const s = getTerminalState();
    expect(s.tabs).toHaveLength(1);
    expect(s.tabs[0].id).toBe('tab-2');
  });

  it('switches active to previous tab when closing active', () => {
    addTab('tab-1');
    addTab('tab-2');
    closeTab('tab-2');
    expect(getTerminalState().activeTabId).toBe('tab-1');
  });

  it('does NOT close the last non-exited tab', () => {
    addTab('tab-1');
    closeTab('tab-1');
    expect(getTerminalState().tabs).toHaveLength(1);
  });

  it('CAN close the last exited tab', () => {
    addTab('tab-1');
    markExited('tab-1');
    closeTab('tab-1');
    expect(getTerminalState().tabs).toHaveLength(0);
  });
});

describe('markExited', () => {
  it('sets exited=true without removing tab', () => {
    addTab('tab-1');
    markExited('tab-1');
    const tab = getTerminalState().tabs.find(t => t.id === 'tab-1');
    expect(tab?.exited).toBe(true);
    expect(getTerminalState().tabs).toHaveLength(1);
  });
});

describe('pendingData', () => {
  it('appendTerminalData accumulates data and drainTerminalData clears it', () => {
    addTab('tab-1');
    appendTerminalData('tab-1', 'hello ');
    appendTerminalData('tab-1', 'world');
    expect(drainTerminalData('tab-1')).toBe('hello world');
    expect(drainTerminalData('tab-1')).toBe('');
  });
});
