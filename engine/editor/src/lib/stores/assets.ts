export interface AssetEntry {
  path: string;
  assetType: 'texture' | 'mesh' | 'audio' | 'config' | 'unknown';
  filename: string;
}

let _assets: AssetEntry[] = [];
let _listeners: ((assets: AssetEntry[]) => void)[] = [];

function _notify() {
  _listeners.forEach((fn) => fn([..._assets]));
}

export function setAssets(list: AssetEntry[]): void {
  _assets = list;
  _notify();
}

export function getAssets(): AssetEntry[] {
  return [..._assets];
}

export function clearAssets(): void {
  _assets = [];
  _notify();
}

export function subscribeAssets(fn: (assets: AssetEntry[]) => void): () => void {
  _listeners.push(fn);
  return () => {
    _listeners = _listeners.filter((l) => l !== fn);
  };
}
