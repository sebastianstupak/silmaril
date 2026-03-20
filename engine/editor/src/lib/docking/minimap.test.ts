import { describe, it, expect } from 'vitest';
import { buildMinimap, buildIcon } from './minimap';
import type { EditorLayout } from './types';

// ── Fixtures ──────────────────────────────────────────────────────────────────

const singlePanel: EditorLayout = {
  root: { type: 'tabs', activeTab: 0, panels: ['viewport'] },
  bottomPanel: { type: 'tabs', activeTab: 0, panels: [] },
};

const threeColumn: EditorLayout = {
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

const nestedSplit: EditorLayout = {
  root: {
    type: 'split', direction: 'horizontal', sizes: [20, 80],
    children: [
      {
        type: 'split', direction: 'vertical', sizes: [60, 40],
        children: [
          { type: 'tabs', activeTab: 0, panels: ['hierarchy'] },
          { type: 'tabs', activeTab: 0, panels: ['assets'] },
        ],
      },
      { type: 'tabs', activeTab: 0, panels: ['viewport'] },
    ],
  },
  bottomPanel: { type: 'tabs', activeTab: 0, panels: [] },
};

// Helper: parse rects out of an SVG string
function parseRects(svg: string): Array<Record<string, string>> {
  const parser = new DOMParser();
  const doc = parser.parseFromString(svg, 'image/svg+xml');
  return Array.from(doc.querySelectorAll('rect')).map(r => ({
    x: r.getAttribute('x') ?? '',
    y: r.getAttribute('y') ?? '',
    width: r.getAttribute('width') ?? '',
    height: r.getAttribute('height') ?? '',
  }));
}

// ── buildMinimap ──────────────────────────────────────────────────────────────

describe('buildMinimap', () => {
  it('returns a valid SVG string', () => {
    const svg = buildMinimap(singlePanel);
    expect(svg).toMatch(/^<svg /);
    expect(svg).toContain('</svg>');
  });

  it('uses the default 120×72 viewBox', () => {
    const svg = buildMinimap(singlePanel);
    expect(svg).toContain('viewBox="0 0 120 72"');
    expect(svg).toContain('width="120"');
    expect(svg).toContain('height="72"');
  });

  it('renders two rects per tabs node (panel body + tab indicator)', () => {
    const svg = buildMinimap(singlePanel);
    const rects = parseRects(svg);
    expect(rects).toHaveLength(2); // body + tab bar
  });

  it('renders the correct number of rects for a 3-column layout with bottom panel', () => {
    const svg = buildMinimap(threeColumn);
    const rects = parseRects(svg);
    // 3 root panels + 1 bottom panel = 4 tabs nodes × 2 rects each = 8
    expect(rects).toHaveLength(8);
  });

  it('takes the full height when the bottom panel is empty', () => {
    const svg = buildMinimap(singlePanel);
    const rects = parseRects(svg);
    const body = rects[0];
    // height should be close to 72 (full height minus padding)
    expect(parseFloat(body.height)).toBeGreaterThan(68);
  });

  it('allocates less height to root when bottom panel has content', () => {
    const withBottom = buildMinimap(threeColumn);
    const noBottom  = buildMinimap(singlePanel);

    const withBottomRects = parseRects(withBottom);
    const noBottomRects   = parseRects(noBottom);

    // The first panel body in the 3-col layout should be shorter than in single-panel
    const threeColHeight = parseFloat(withBottomRects[0].height);
    const fullHeight     = parseFloat(noBottomRects[0].height);
    expect(threeColHeight).toBeLessThan(fullHeight);
  });

  it('handles nested splits without crashing', () => {
    expect(() => buildMinimap(nestedSplit)).not.toThrow();
    const svg = buildMinimap(nestedSplit);
    expect(svg).toContain('<svg');
  });

  it('horizontal split places children side-by-side (different x offsets)', () => {
    const svg = buildMinimap(threeColumn);
    const rects = parseRects(svg);
    // First rect of each panel body: x coords should increase left-to-right
    const bodyRects = rects.filter((_, i) => i % 2 === 0).slice(0, 3);
    const xs = bodyRects.map(r => parseFloat(r.x));
    expect(xs[0]).toBeLessThan(xs[1]);
    expect(xs[1]).toBeLessThan(xs[2]);
  });
});

// ── buildIcon ─────────────────────────────────────────────────────────────────

describe('buildIcon', () => {
  it('returns a valid SVG string', () => {
    const svg = buildIcon(singlePanel, 16, 11);
    expect(svg).toMatch(/^<svg /);
    expect(svg).toContain('</svg>');
  });

  it('uses the requested width and height', () => {
    const svg = buildIcon(singlePanel, 16, 11);
    expect(svg).toContain('width="16"');
    expect(svg).toContain('height="11"');
    expect(svg).toContain('viewBox="0 0 16 11"');
  });

  it('renders one rect per tabs node (no tab indicator, simpler than minimap)', () => {
    const svg = buildIcon(singlePanel, 16, 11);
    const rects = parseRects(svg);
    expect(rects).toHaveLength(1);
  });

  it('renders one rect per panel for a 3-column layout', () => {
    const svg = buildIcon(threeColumn, 16, 11);
    const rects = parseRects(svg);
    // 3 root + 1 bottom
    expect(rects).toHaveLength(4);
  });

  it('works at any requested size', () => {
    expect(() => buildIcon(singlePanel, 100, 60)).not.toThrow();
    expect(() => buildIcon(singlePanel, 8, 6)).not.toThrow();
  });
});
