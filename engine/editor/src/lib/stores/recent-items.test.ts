import { describe, it, expect, beforeEach } from 'vitest';
import {
  addRecentItem,
  getRecentItems,
  subscribeRecent,
  _resetRecentItems,
} from './recent-items';

beforeEach(() => _resetRecentItems());

describe('addRecentItem', () => {
  it('adds an item', () => {
    addRecentItem({ label: 'My Game', path: '/path/my-game', itemType: 'project' });
    expect(getRecentItems()).toHaveLength(1);
    expect(getRecentItems()[0].label).toBe('My Game');
  });

  it('deduplicates by path — re-opening moves item to front', () => {
    addRecentItem({ label: 'A', path: '/a', itemType: 'project' });
    addRecentItem({ label: 'B', path: '/b', itemType: 'project' });
    addRecentItem({ label: 'A2', path: '/a', itemType: 'project' });
    const items = getRecentItems();
    expect(items).toHaveLength(2);
    expect(items[0].path).toBe('/a');
    expect(items[0].label).toBe('A2'); // label updated
  });

  it('caps at 10 items, dropping oldest', () => {
    for (let i = 0; i < 12; i++) {
      addRecentItem({ label: `P${i}`, path: `/p${i}`, itemType: 'project' });
    }
    expect(getRecentItems()).toHaveLength(10);
    // newest items survive
    expect(getRecentItems()[0].path).toBe('/p11');
  });
});

describe('subscribeRecent', () => {
  it('notifies subscriber when item added', () => {
    let called = 0;
    const unsub = subscribeRecent(() => { called++; });
    addRecentItem({ label: 'X', path: '/x', itemType: 'project' });
    expect(called).toBe(1);
    unsub();
    addRecentItem({ label: 'Y', path: '/y', itemType: 'project' });
    expect(called).toBe(1); // unsubscribed
  });
});
