/**
 * Playwright E2E tests for Editor Layout Presets & Dynamic Panel Registry.
 *
 * Tests what is observable in browser/preview mode:
 *   1. Scene preset panel structure on initial load
 *   2. Dock tab labels come from the ContributionRegistry (correct titles)
 *   3. All expected panels are reachable and render their content
 *
 * NOTE: TitleBar-specific features (layout slot switcher, panel spawn menu,
 * AI server indicator) are only rendered inside the Tauri shell
 * (`{#if isTauri}` in App.svelte) and therefore cannot be tested here.
 * These features are verified via the Tauri integration build.
 *
 * Run against `npm run preview` (built app on port 4173).
 */
import { test, expect, type Page } from '@playwright/test';
import * as path from 'path';

function shot(name: string) {
  return path.join('e2e', 'screenshots', `panels-${name}.png`);
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Returns the text of all visible dock tab labels. */
async function getTabLabels(page: Page): Promise<string[]> {
  // DockTabBar renders [role="tab"] buttons
  const tabs = page.locator('[role="tab"]');
  const count = await tabs.count();
  const labels: string[] = [];
  for (let i = 0; i < count; i++) {
    const text = (await tabs.nth(i).textContent())?.trim() ?? '';
    if (text) labels.push(text);
  }
  return labels;
}

// ---------------------------------------------------------------------------
// Suite
// ---------------------------------------------------------------------------

test.describe('Panel Registry & Layout Presets', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await page.waitForSelector('.hierarchy', { timeout: 15_000 });
  });

  // ─── 1. Scene preset structure on initial load ────────────────────────────

  test('initial layout contains Hierarchy, Viewport, Inspector panels', async ({ page }) => {
    // All three primary panels must have their tab headers visible
    await expect(page.locator('[role="tab"]', { hasText: /hierarchy/i })).toBeVisible();
    await expect(page.locator('[role="tab"]', { hasText: /viewport/i })).toBeVisible();
    await expect(page.locator('[role="tab"]', { hasText: /inspector/i })).toBeVisible();

    await page.screenshot({ path: shot('01-primary-panels') });
  });

  test('initial layout contains Console and Output in the bottom panel', async ({ page }) => {
    await expect(page.locator('[role="tab"]', { hasText: /console/i })).toBeVisible();
    await expect(page.locator('[role="tab"]', { hasText: /output/i })).toBeVisible();

    await page.screenshot({ path: shot('02-bottom-panels') });
  });

  test('initial layout contains Assets and File Explorer tabs', async ({ page }) => {
    // Scene preset has [assets, file-explorer] in the lower-left slot
    await expect(page.locator('[role="tab"]', { hasText: /assets/i })).toBeVisible();
    await expect(page.locator('[role="tab"]', { hasText: /file.?explorer/i })).toBeVisible();

    await page.screenshot({ path: shot('03-asset-panels') });
  });

  // ─── 2. Tab labels from ContributionRegistry ─────────────────────────────

  test('dock tabs are titled from the registry — no raw IDs like "viewport:0"', async ({ page }) => {
    const labels = await getTabLabels(page);
    expect(labels.length).toBeGreaterThan(0);

    // None of the tab labels should look like a raw panel instance ID (e.g. "viewport:0")
    for (const label of labels) {
      expect(label).not.toMatch(/^[a-z-]+:\d+$/);
    }

    // At minimum the 5 panels in the Scene preset bottom bar + primary area
    // should all have human-readable titles
    const lowerLabels = labels.map(l => l.toLowerCase());
    expect(lowerLabels.some(l => l.includes('hierarchy'))).toBe(true);
    expect(lowerLabels.some(l => l.includes('viewport'))).toBe(true);
    expect(lowerLabels.some(l => l.includes('inspector'))).toBe(true);
    expect(lowerLabels.some(l => l.includes('console'))).toBe(true);

    await page.screenshot({ path: shot('04-tab-labels') });
  });

  // ─── 3. Panel components actually render ─────────────────────────────────

  test('Hierarchy panel renders entity list and add button', async ({ page }) => {
    const hierarchyTab = page.locator('[role="tab"]', { hasText: /hierarchy/i }).first();
    await hierarchyTab.click();

    await expect(page.locator('.hierarchy')).toBeVisible();
    await expect(page.locator('button[title="New Entity"]')).toBeVisible();

    await page.screenshot({ path: shot('05-hierarchy-panel') });
  });

  test('Inspector panel renders "No entity selected" placeholder', async ({ page }) => {
    const inspectorTab = page.locator('[role="tab"]', { hasText: /inspector/i }).first();
    await inspectorTab.click();

    await expect(page.locator('.inspector')).toBeVisible();
    // Default state: nothing selected
    await expect(page.locator('.inspector')).toContainText(/no entity selected/i);

    await page.screenshot({ path: shot('06-inspector-panel') });
  });

  test('Console panel renders log toolbar', async ({ page }) => {
    const consoleTab = page.locator('[role="tab"]', { hasText: /console/i }).first();
    await consoleTab.click();

    // Console has filter buttons for Info / Warn / Error / Debug
    await expect(page.locator('button', { hasText: /info/i }).first()).toBeVisible();

    await page.screenshot({ path: shot('07-console-panel') });
  });

  test('clicking Assets tab switches to the Assets panel', async ({ page }) => {
    const assetsTab = page.locator('[role="tab"]', { hasText: /assets/i }).first();
    await assetsTab.click();

    // After clicking, the Assets tab should be active (aria-selected or active class)
    await expect(assetsTab).toHaveAttribute('aria-selected', 'true');

    await page.screenshot({ path: shot('08-assets-tab-active') });
  });

  test('clicking File Explorer tab switches to the File Explorer panel', async ({ page }) => {
    const fileExplorerTab = page.locator('[role="tab"]', { hasText: /file.?explorer/i }).first();
    await fileExplorerTab.click();

    await expect(fileExplorerTab).toHaveAttribute('aria-selected', 'true');

    await page.screenshot({ path: shot('09-file-explorer-tab-active') });
  });

  // ─── 4. Bottom panel integration ─────────────────────────────────────────

  test('switching between Console and Output tabs works', async ({ page }) => {
    const consoleTab = page.locator('[role="tab"]', { hasText: /console/i }).first();
    const outputTab  = page.locator('[role="tab"]', { hasText: /output/i }).first();

    await consoleTab.click();
    await expect(consoleTab).toHaveAttribute('aria-selected', 'true');

    await outputTab.click();
    await expect(outputTab).toHaveAttribute('aria-selected', 'true');

    await page.screenshot({ path: shot('10-console-output-switch') });
  });
});
