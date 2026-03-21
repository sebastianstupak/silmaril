import { describe, it, expect, beforeEach } from 'vitest';
import { setAssets, getAssets, clearAssets, subscribeAssets } from './assets';

beforeEach(() => clearAssets());

describe('setAssets / getAssets', () => {
  it('stores and retrieves assets', () => {
    setAssets([{ path: '/a/player.png', assetType: 'texture', filename: 'player.png' }]);
    expect(getAssets()).toHaveLength(1);
    expect(getAssets()[0].filename).toBe('player.png');
  });

  it('replaces previous list on setAssets', () => {
    setAssets([{ path: '/a.png', assetType: 'texture', filename: 'a.png' }]);
    setAssets([{ path: '/b.glb', assetType: 'mesh', filename: 'b.glb' },
               { path: '/c.glb', assetType: 'mesh', filename: 'c.glb' }]);
    expect(getAssets()).toHaveLength(2);
  });
});

describe('clearAssets', () => {
  it('empties the list', () => {
    setAssets([{ path: '/x.wav', assetType: 'audio', filename: 'x.wav' }]);
    clearAssets();
    expect(getAssets()).toHaveLength(0);
  });
});

describe('subscribeAssets', () => {
  it('notifies on setAssets and clearAssets', () => {
    let count = 0;
    const unsub = subscribeAssets(() => { count++; });
    setAssets([]);
    clearAssets();
    expect(count).toBe(2);
    unsub();
    setAssets([]);
    expect(count).toBe(2); // no more notifications
  });
});
