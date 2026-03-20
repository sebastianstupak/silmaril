import type { EditorLayout, LayoutNode } from './types';

function minimapNode(node: LayoutNode, x: number, y: number, w: number, h: number): string {
  if (node.type === 'tabs') {
    const p = 1.5;
    return (
      `<rect x="${(x+p).toFixed(1)}" y="${(y+p).toFixed(1)}" width="${Math.max(0,w-p*2).toFixed(1)}" height="${Math.max(0,h-p*2).toFixed(1)}" rx="2" fill="currentColor" opacity="0.2"/>` +
      `<rect x="${(x+p+1).toFixed(1)}" y="${(y+p+1).toFixed(1)}" width="${Math.min(w*0.45,22).toFixed(1)}" height="2.5" rx="1" fill="currentColor" opacity="0.45"/>`
    );
  }
  const isH = node.direction === 'horizontal';
  const total = isH ? w : h;
  let out = '';
  let off = 0;
  for (let i = 0; i < node.children.length; i++) {
    const sz = (node.sizes[i] / 100) * total;
    out += minimapNode(
      node.children[i],
      isH ? x + off : x,
      isH ? y : y + off,
      isH ? sz : w,
      isH ? h : sz,
    );
    off += sz;
  }
  return out;
}

function hasContent(node: LayoutNode): boolean {
  if (node.type === 'tabs') return node.panels.length > 0;
  return node.children.length > 0;
}

/** Full hover-card minimap: 120×72, two rects per tabs node (body + tab bar). */
export function buildMinimap(layout: EditorLayout): string {
  const W = 120, H = 72;
  const bottom = hasContent(layout.bottomPanel);
  const mainH = bottom ? H * 0.72 : H;
  let inner = minimapNode(layout.root, 0, 0, W, mainH);
  if (bottom) inner += minimapNode(layout.bottomPanel, 0, mainH + 1, W, H - mainH - 1);
  return `<svg width="120" height="72" viewBox="0 0 120 72" xmlns="http://www.w3.org/2000/svg">${inner}</svg>`;
}

/** Small schematic icon for slot buttons: one filled rect per tabs node, no tab indicator. */
export function buildIcon(layout: EditorLayout, W: number, H: number): string {
  function iconNode(node: LayoutNode, x: number, y: number, w: number, h: number): string {
    if (node.type === 'tabs') {
      const p = 0.5;
      return `<rect x="${(x+p).toFixed(1)}" y="${(y+p).toFixed(1)}" width="${Math.max(0,w-p*2).toFixed(1)}" height="${Math.max(0,h-p*2).toFixed(1)}" rx="0.5" fill="currentColor"/>`;
    }
    const isH = node.direction === 'horizontal';
    const total = isH ? w : h;
    let out = '';
    let off = 0;
    for (let i = 0; i < node.children.length; i++) {
      const sz = (node.sizes[i] / 100) * total;
      out += iconNode(
        node.children[i],
        isH ? x + off : x,
        isH ? y : y + off,
        isH ? sz : w,
        isH ? h : sz,
      );
      off += sz;
    }
    return out;
  }
  const bottom = hasContent(layout.bottomPanel);
  const mainH = bottom ? H * 0.72 : H;
  let inner = iconNode(layout.root, 0, 0, W, mainH);
  if (bottom) inner += iconNode(layout.bottomPanel, 0, mainH + 1, W, H - mainH - 1);
  return `<svg width="${W}" height="${H}" viewBox="0 0 ${W} ${H}" xmlns="http://www.w3.org/2000/svg">${inner}</svg>`;
}
