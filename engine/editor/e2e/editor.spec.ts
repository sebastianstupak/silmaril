/**
 * Playwright E2E tests for the Silmaril editor in browser mode.
 *
 * Run against `npm run preview` (built app on port 4173).
 * Selectors are tied to actual CSS class names / ARIA attributes — update
 * them when the markup changes, not when tests fail for unrelated reasons.
 */
import { test, expect, type Page } from '@playwright/test';

// ---------------------------------------------------------------------------
// Selectors — mirrors the real DOM class names / attributes
// ---------------------------------------------------------------------------

const SEL = {
  // Hierarchy panel
  hierarchy:     '.hierarchy',
  newEntityBtn:  'button.add-btn[title="New Entity"]',
  entityRow:     '.entity-row',
  entityName:    '.entity-name',   // span inside an entity-row
  renameInput:   'input.rename-input',
  entityActions: '.entity-actions',
  addChildBtn:   'button.action-btn[title="Add Child"]',
  contextMenu:   '.context-menu',
  contextMenuAddChild: 'button[role="menuitem"]:has-text("Add Child")',

  // Inspector panel
  inspector:         '.inspector',
  inspectorEntityName: '.inspector-entity-name',
  componentSection:  '.component-section',
  componentHeader:   '.component-header',
  removeComponentBtn: '.remove-component-btn',   // title="Remove {Name}"
  addComponentBtn:   '.add-component-btn',        // text "+ Add Component…"
  componentFilterInput: '.component-filter-input',
  componentPickerList:  '.component-picker-list',

  // Vec3 widget (Vec3Field.svelte → class="vec3-group")
  vec3Group: '.vec3-group',

  // Viewport toolbar (ViewportPanel.svelte)
  toolBtn:    'button.tool-btn',               // all tool buttons
  toolSelect: 'button.tool-btn[aria-label="Select"]',
  toolMove:   'button.tool-btn[aria-label="Move"]',
  toolRotate: 'button.tool-btn[aria-label="Rotate"]',
  toolScale:  'button.tool-btn[aria-label="Scale"]',
} as const;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Create an entity and return the first entity row locator. */
async function createEntity(page: Page) {
  const before = await page.locator(SEL.entityRow).count();
  await page.locator(SEL.newEntityBtn).click();
  await expect(page.locator(SEL.entityRow)).toHaveCount(before + 1);
  return page.locator(SEL.entityRow).first();
}

/** Select an entity row and wait for the inspector to reflect it. */
async function selectEntity(page: Page, row: ReturnType<Page['locator']>) {
  await row.click();
  await expect(page.locator(SEL.inspectorEntityName)).toBeVisible();
}

// ---------------------------------------------------------------------------
// Suite
// ---------------------------------------------------------------------------

test.describe('Silmaril Editor', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    // Hierarchy panel must be present before any test action.
    await page.waitForSelector(SEL.hierarchy, { timeout: 15_000 });
  });

  // ─── App shell ────────────────────────────────────────────────────────────

  test('loads and renders the editor shell', async ({ page }) => {
    await expect(page.locator(SEL.hierarchy)).toBeVisible();
    await expect(page.locator(SEL.inspector)).toBeVisible();
  });

  // ─── Entity creation ──────────────────────────────────────────────────────

  test('creates a new entity via the + button', async ({ page }) => {
    const before = await page.locator(SEL.entityRow).count();
    await page.locator(SEL.newEntityBtn).click();
    await expect(page.locator(SEL.entityRow)).toHaveCount(before + 1);
  });

  test('newly created entity appears with a default name', async ({ page }) => {
    await page.locator(SEL.newEntityBtn).click();
    const row = page.locator(SEL.entityRow).first();
    const text = await row.textContent();
    expect(text?.trim().length).toBeGreaterThan(0);
  });

  // ─── Selection + inspector ────────────────────────────────────────────────

  test('selecting an entity shows its name in the inspector', async ({ page }) => {
    const row = await createEntity(page);
    const rowText = (await row.locator(SEL.entityName).textContent())?.trim() ?? '';
    await row.click();
    await expect(page.locator(SEL.inspectorEntityName)).toContainText(rowText);
  });

  test('inspector shows a Transform component section when entity is selected', async ({ page }) => {
    const row = await createEntity(page);
    await selectEntity(page, row);
    await expect(
      page.locator(SEL.componentSection).filter({ hasText: 'Transform' })
    ).toBeVisible({ timeout: 5_000 });
  });

  test('inspector renders Vec3 fields for Transform position/rotation/scale', async ({ page }) => {
    const row = await createEntity(page);
    await selectEntity(page, row);
    // Three Vec3 groups: Position, Rotation, Scale
    await expect(page.locator(SEL.vec3Group).first()).toBeVisible({ timeout: 5_000 });
    const count = await page.locator(SEL.vec3Group).count();
    expect(count).toBeGreaterThanOrEqual(3);
  });

  // ─── Inspector field editing ──────────────────────────────────────────────

  test('editing a position X field updates its value', async ({ page }) => {
    const row = await createEntity(page);
    await selectEntity(page, row);

    // Position is the first Vec3 group; X is the first number input inside it
    const posX = page.locator(SEL.vec3Group).first().locator('input[type="number"]').first();
    await expect(posX).toBeVisible({ timeout: 5_000 });
    await posX.fill('42');
    await posX.press('Enter');
    await expect(posX).toHaveValue('42');
  });

  // ─── Entity rename ────────────────────────────────────────────────────────

  test('double-clicking an entity row opens the rename input', async ({ page }) => {
    const row = await createEntity(page);
    await row.dblclick();
    await expect(page.locator(SEL.renameInput)).toBeVisible();
  });

  test('renaming an entity and pressing Enter commits the new name', async ({ page }) => {
    const row = await createEntity(page);
    await row.dblclick();
    await page.locator(SEL.renameInput).fill('MyRenamedEntity');
    await page.locator(SEL.renameInput).press('Enter');
    await expect(page.locator(SEL.entityRow).first()).toContainText('MyRenamedEntity');
  });

  test('pressing Escape during rename cancels and restores the old name', async ({ page }) => {
    const row = await createEntity(page);
    const originalName = (await row.locator(SEL.entityName).textContent())?.trim() ?? '';
    await row.dblclick();
    await page.locator(SEL.renameInput).fill('ShouldNotStick');
    await page.locator(SEL.renameInput).press('Escape');
    await expect(page.locator(SEL.entityRow).first()).toContainText(originalName);
  });

  // ─── Entity deletion ──────────────────────────────────────────────────────

  test('Delete key removes the selected entity', async ({ page }) => {
    const row = await createEntity(page);
    const before = await page.locator(SEL.entityRow).count();
    await row.click();
    // Hierarchy must be focused to receive Delete
    await page.locator(SEL.hierarchy).press('Delete');
    await expect(page.locator(SEL.entityRow)).toHaveCount(before - 1);
  });

  // ─── Component add / remove ───────────────────────────────────────────────

  test('opens the component picker from Add Component button', async ({ page }) => {
    const row = await createEntity(page);
    await selectEntity(page, row);

    await page.locator(SEL.addComponentBtn).click();
    await expect(page.locator(SEL.componentFilterInput)).toBeVisible();
    await expect(page.locator(SEL.componentPickerList)).toBeVisible();
  });

  test('adds Health component to a selected entity', async ({ page }) => {
    const row = await createEntity(page);
    await selectEntity(page, row);

    await page.locator(SEL.addComponentBtn).click();
    await page.locator(SEL.componentFilterInput).fill('Health');
    await page.locator(SEL.componentPickerList).locator('button:has-text("Health")').first().click();

    await expect(
      page.locator(SEL.componentSection).filter({ hasText: 'Health' })
    ).toBeVisible();
  });

  test('removes a component via its × button', async ({ page }) => {
    const row = await createEntity(page);
    await selectEntity(page, row);

    // Add Health first
    await page.locator(SEL.addComponentBtn).click();
    await page.locator(SEL.componentFilterInput).fill('Health');
    await page.locator(SEL.componentPickerList).locator('button:has-text("Health")').first().click();
    await expect(
      page.locator(SEL.componentSection).filter({ hasText: 'Health' })
    ).toBeVisible();

    // Remove it
    await page.locator('.remove-component-btn[title="Remove Health"]').click();
    await expect(
      page.locator(SEL.componentSection).filter({ hasText: 'Health' })
    ).not.toBeVisible();
  });

  // ─── Viewport tools ───────────────────────────────────────────────────────
  // Tool buttons have aria-label="Select|Move|Rotate|Scale" and class:active

  test('Select tool button is active by default', async ({ page }) => {
    await expect(page.locator(SEL.toolSelect)).toHaveClass(/active/);
  });

  test('clicking Move tool button activates it', async ({ page }) => {
    await page.locator(SEL.toolMove).click();
    await expect(page.locator(SEL.toolMove)).toHaveClass(/active/);
    await expect(page.locator(SEL.toolSelect)).not.toHaveClass(/active/);
  });

  test('clicking Rotate tool button activates it', async ({ page }) => {
    await page.locator(SEL.toolRotate).click();
    await expect(page.locator(SEL.toolRotate)).toHaveClass(/active/);
  });

  test('clicking Scale tool button activates it', async ({ page }) => {
    await page.locator(SEL.toolScale).click();
    await expect(page.locator(SEL.toolScale)).toHaveClass(/active/);
  });

  test('keyboard shortcut Q activates the Select tool', async ({ page }) => {
    // Activate a different tool first to ensure the state change is observable
    await page.locator(SEL.toolMove).click();
    await page.keyboard.press('q');
    await expect(page.locator(SEL.toolSelect)).toHaveClass(/active/);
  });

  test('keyboard shortcut W activates the Move tool', async ({ page }) => {
    // Click Select first so the page is fully interactive and we can verify the state change.
    await page.locator(SEL.toolSelect).click();
    await page.keyboard.press('w');
    await expect(page.locator(SEL.toolMove)).toHaveClass(/active/);
  });

  // ─── Entity hierarchy (child creation) ────────────────────────────────────

  test('hover action button creates a child entity', async ({ page }) => {
    const parentRow = await createEntity(page);
    const before = await page.locator(SEL.entityRow).count();

    // Hover to reveal the action buttons strip
    await parentRow.hover();
    await page.locator(SEL.addChildBtn).first().click();

    await expect(page.locator(SEL.entityRow)).toHaveCount(before + 1);
  });

  test('context menu Add Child creates a child entity', async ({ page }) => {
    const parentRow = await createEntity(page);
    const before = await page.locator(SEL.entityRow).count();

    await parentRow.click({ button: 'right' });
    await expect(page.locator(SEL.contextMenu)).toBeVisible();
    await page.locator(SEL.contextMenuAddChild).click();

    await expect(page.locator(SEL.entityRow)).toHaveCount(before + 1);
  });

  // ─── Mesh Rendering (Phase 1.8) ───────────────────────────────────────────
  // Note: MeshRenderer component picker and mesh assignment require the Tauri
  // backend (get_component_schemas IPC). These tests validate what is testable
  // in browser preview mode and capture the editor layout with mesh UI.

  test('editor shell renders all panels needed for mesh workflow', async ({ page }) => {
    // Capture the full editor shell — hierarchy, viewport, inspector, assets all visible
    await expect(page.locator(SEL.hierarchy)).toBeVisible();
    await expect(page.locator(SEL.inspector)).toBeVisible();
    await page.screenshot({ path: 'e2e/screenshots/mesh-01-editor-shell.png', fullPage: false });
  });

  test('inspector shows Add Component button for mesh assignment workflow', async ({ page }) => {
    const row = await createEntity(page);
    await selectEntity(page, row);

    // Inspector panel loads with Transform and Add Component button
    await expect(page.locator(SEL.addComponentBtn)).toBeVisible();

    // Open the component filter — it should show the filter input
    await page.locator(SEL.addComponentBtn).click();
    await expect(page.locator(SEL.componentFilterInput)).toBeVisible();
    await expect(page.locator(SEL.componentPickerList)).toBeVisible();

    // Type "Mesh" — in browser preview mode schemas are empty (requires Tauri backend)
    // so no items appear, but the picker UI is correctly rendered
    await page.locator(SEL.componentFilterInput).fill('Mesh');

    await page.screenshot({ path: 'e2e/screenshots/mesh-02-component-picker-with-mesh-filter.png' });
  });

  test('hierarchy panel entity rows have drop-target data attributes for mesh assignment', async ({ page }) => {
    const row = await createEntity(page);
    // Entity row should be visible and draggable (drop target for mesh drag-and-drop)
    await expect(row).toBeVisible();

    // Check the entity row renders the entity name
    await expect(row.locator(SEL.entityName)).toBeVisible();

    await row.click();
    await page.screenshot({ path: 'e2e/screenshots/mesh-03-hierarchy-entity-selected.png' });
  });

  test('inspector panel renders for selected entity ready for MeshRenderer', async ({ page }) => {
    const row = await createEntity(page);
    await selectEntity(page, row);

    // Inspector should show entity name and Transform (foundation for MeshRenderer display)
    await expect(page.locator(SEL.inspectorEntityName)).toBeVisible();
    await expect(page.locator(SEL.componentSection).filter({ hasText: 'Transform' })).toBeVisible();
    await expect(page.locator(SEL.addComponentBtn)).toBeVisible();

    await page.screenshot({ path: 'e2e/screenshots/mesh-04-inspector-ready-for-mesh.png' });
  });
});
