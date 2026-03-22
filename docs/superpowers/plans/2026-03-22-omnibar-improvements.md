# Omnibar Improvements — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Four omnibar improvements — wider responsive modal, prefix hints when empty, grouped results with section headers, and dropdown positioned below the titlebar.

**Architecture:** Frontend-only changes across Omnibar.svelte, types.ts, providers.ts, and TitleBar.svelte. No backend IPC changes. CSS custom property --titlebar-height added to TitleBar.svelte as single source of truth for positioning.

**Tech Stack:** Svelte 5 (runes), TypeScript, CSS

---

## Task 1 — Add `--titlebar-height` CSS custom property to TitleBar.svelte

**File:** `engine/editor/src/lib/components/TitleBar.svelte`

**Where:** The `.titlebar` CSS rule starts at line 643.

**Change:** Insert `--titlebar-height: 32px;` as the first property inside the `.titlebar` rule, before the existing `height: 32px;`. This makes the height a single-source-of-truth custom property consumed by both `.titlebar` itself and later by Omnibar.svelte.

**Current block (lines 643–653):**
```css
.titlebar {
  height: 32px;
  display: grid;
  grid-template-columns: 1fr auto 1fr;
  align-items: center;
  background: var(--color-bgTitleBar, #141414);
  border-bottom: 1px solid color-mix(in srgb, var(--color-border, #404040) 60%, transparent);
  user-select: none;
  -webkit-user-select: none;
  flex-shrink: 0;
  cursor: default;
```

**New block:**
```css
.titlebar {
  --titlebar-height: 32px;
  height: var(--titlebar-height);
  display: grid;
  grid-template-columns: 1fr auto 1fr;
  align-items: center;
  background: var(--color-bgTitleBar, #141414);
  border-bottom: 1px solid color-mix(in srgb, var(--color-border, #404040) 60%, transparent);
  user-select: none;
  -webkit-user-select: none;
  flex-shrink: 0;
  cursor: default;
```

**Steps:**
- [ ] Edit `engine/editor/src/lib/components/TitleBar.svelte`: replace `height: 32px;` with `--titlebar-height: 32px;` on the first line of `.titlebar`, then add `height: var(--titlebar-height);` on the next line.
- [ ] Visual check: `npm run preview` from `engine/editor/` — titlebar height must remain 32px, no visual regression.
- [ ] Commit: `feat(editor): add --titlebar-height CSS custom property to .titlebar`

---

## Task 2 — Add `group?: string` to every arm of `OmnibarResult` in types.ts

**File:** `engine/editor/src/lib/omnibar/types.ts`

**Where:** Lines 26–30 — the `OmnibarResult` discriminated union.

**Current:**
```ts
export type OmnibarResult =
  | { kind: 'command'; command: AnyCommand }
  | { kind: 'entity'; id: number; name: string; components: string[] }
  | { kind: 'asset'; path: string; assetType: string }
  | { kind: 'recent'; label: string; path: string; itemType: 'project' | 'scene' };
```

**New (add `group?: string` to each arm — do not flatten or restructure):**
```ts
export type OmnibarResult =
  | { kind: 'command'; command: AnyCommand; group?: string }
  | { kind: 'entity'; id: number; name: string; components: string[]; group?: string }
  | { kind: 'asset'; path: string; assetType: string; group?: string }
  | { kind: 'recent'; label: string; path: string; itemType: 'project' | 'scene'; group?: string };
```

**Steps:**
- [ ] Edit `engine/editor/src/lib/omnibar/types.ts`: add `group?: string` to each arm of the union as shown above.
- [ ] Run `npm run test` from `engine/editor/` — TypeScript must compile cleanly, no existing tests may fail.
- [ ] Commit: `feat(editor): add group? field to OmnibarResult discriminated union`

---

## Task 3 — Set `group` on each result in `buildResults()` in providers.ts

**File:** `engine/editor/src/lib/omnibar/providers.ts`

**Where:** Lines 46–71 — the `buildResults` function body.

**Current function body (lines 46–72):**
```ts
  const { prefix, query: q } = parsePrefix(query);
  const results: OmnibarResult[] = [];

  if (!prefix || prefix === 'command') {
    for (const cmd of filterCommandResults(commands, q)) {
      results.push({ kind: 'command', command: cmd });
    }
  }
  if (!prefix || prefix === 'entity') {
    for (const e of filterEntityResults(entities, q)) {
      results.push({ kind: 'entity', ...e });
    }
  }
  if (!prefix || prefix === 'asset') {
    for (const a of filterAssetResults(assets, q)) {
      results.push({ kind: 'asset', ...a });
    }
  }
  if (!prefix && !q) {
    // Empty input: show recent
    for (const r of recent) {
      results.push({ kind: 'recent', ...r });
    }
  }

  return results;
```

**New function body — add `group` inline at each push site:**
```ts
  const { prefix, query: q } = parsePrefix(query);
  const results: OmnibarResult[] = [];

  if (!prefix || prefix === 'command') {
    for (const cmd of filterCommandResults(commands, q)) {
      results.push({ kind: 'command', command: cmd, group: 'Commands' });
    }
  }
  if (!prefix || prefix === 'entity') {
    for (const e of filterEntityResults(entities, q)) {
      results.push({ kind: 'entity', ...e, group: 'Entities' });
    }
  }
  if (!prefix || prefix === 'asset') {
    for (const a of filterAssetResults(assets, q)) {
      results.push({ kind: 'asset', ...a, group: 'Assets' });
    }
  }
  if (!prefix && !q) {
    // Empty input: show recent
    for (const r of recent) {
      results.push({ kind: 'recent', ...r, group: 'Recent' });
    }
  }

  return results;
```

**Design note:** When a prefix filter is active (e.g. `>`), only one group appears in the results array. The `showGroups` derived in Omnibar.svelte (Task 6) will detect this and suppress section headers — there is no need to conditionally omit the `group` field here.

**Steps:**
- [ ] Edit `engine/editor/src/lib/omnibar/providers.ts`: add `group: 'Commands'`, `group: 'Entities'`, `group: 'Assets'`, and `group: 'Recent'` to the four push sites as shown above.
- [ ] Run `npm run test` from `engine/editor/` — all provider unit tests must pass.
- [ ] Commit: `feat(editor): set group field on omnibar results in buildResults`

---

## Task 4 — Fix `.omnibar-active` positioning (fixed, below titlebar, responsive width)

**File:** `engine/editor/src/lib/omnibar/Omnibar.svelte`

**Where:** The `.omnibar-active` CSS rule, lines 268–279.

**Current:**
```css
  .omnibar-active {
    position: absolute;
    top: 0;
    left: 50%;
    transform: translateX(-50%);
    width: 360px;
    z-index: 10000;
    background: var(--color-bgPanel, #1e1e2e);
    border: 1px solid var(--color-accent, #89b4fa);
    border-radius: 6px;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.6);
  }
```

**New:**
```css
  .omnibar-active {
    position: fixed;
    top: calc(var(--titlebar-height, 32px) + 2px);
    left: 50%;
    transform: translateX(-50%);
    width: clamp(360px, 40vw, 580px);
    z-index: 10000;
    background: var(--color-bgPanel, #1e1e2e);
    border: 1px solid var(--color-accent, #89b4fa);
    border-radius: 6px;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.6);
  }
```

**Design notes:**
- `position: fixed` removes the dependency on any ancestor stacking context. The backdrop (`.omnibar-backdrop`) is already `position: fixed; inset: 0` — no change needed there.
- `top: calc(var(--titlebar-height, 32px) + 2px)` reads the custom property set on `.titlebar` (Task 1). The `32px` fallback ensures the modal degrades gracefully if rendered outside the full editor shell.
- `width: clamp(360px, 40vw, 580px)`: at 900px window → 360px; at 1280px → 512px; at 1600px+ → capped at 580px.

**Steps:**
- [ ] Edit `engine/editor/src/lib/omnibar/Omnibar.svelte`: replace the `.omnibar-active` rule as shown above.
- [ ] Visual check: `npm run preview` from `engine/editor/` — open the omnibar (Ctrl+K) and confirm the dropdown appears just below the titlebar with a 2px gap, not overlapping it. Resize the window and verify width responds between 360px and 580px.
- [ ] Run Playwright tests: `npm run test:e2e` from `engine/editor/`. Add the following test to `engine/editor/e2e/editor.spec.ts` before running:
  ```ts
  test('omnibar opens below titlebar on Ctrl+K', async ({ page }) => {
    await page.keyboard.press('Control+k');
    const modal = page.locator('.omnibar-active');
    await expect(modal).toBeVisible();
    const box = await modal.boundingBox();
    expect(box!.y).toBeGreaterThanOrEqual(32);
  });

  test('omnibar width is responsive', async ({ page }) => {
    await page.keyboard.press('Control+k');
    const modal = page.locator('.omnibar-active');
    await expect(modal).toBeVisible();
    const box = await modal.boundingBox();
    expect(box!.width).toBeGreaterThanOrEqual(360);
    expect(box!.width).toBeLessThanOrEqual(580);
  });
  ```
- [ ] Commit: `fix(editor): position omnibar-active fixed below titlebar, responsive width`

---

## Task 5 — Add `PREFIX_HINTS` const + empty-state hint render + CSS in Omnibar.svelte

**File:** `engine/editor/src/lib/omnibar/Omnibar.svelte`

### 5a — Add `PREFIX_HINTS` constant in the `<script>` block

**Where:** After the imports and before the `interface Props` declaration (around line 17). Insert the constant as a module-level `const` (not inside a component function):

```ts
const PREFIX_HINTS = [
  { prefix: '>',  label: 'Commands',       description: 'Run editor commands' },
  { prefix: '@',  label: 'Scene entities', description: 'Find entities in the scene' },
  { prefix: '#',  label: 'Assets',         description: 'Find project assets' },
] as const;
```

### 5b — Replace the active-state template with conditional empty/results render

**Where:** Lines 176–221 — the `{:else}` branch (active state) of the `{#if !open}` block.

**Current template (active state):**
```svelte
  {:else}
    <!-- Active state -->
    <div class="omnibar-active">
      <div class="omnibar-input-row">
        <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor" class="omnibar-icon" aria-hidden="true">
          <circle cx="6" cy="6" r="4.5" stroke="currentColor" stroke-width="1.5" fill="none"/>
          <line x1="9.5" y1="9.5" x2="13" y2="13" stroke="currentColor" stroke-width="1.5"/>
        </svg>
        <input
          bind:this={inputEl}
          bind:value={query}
          type="text"
          class="omnibar-input"
          placeholder="Search commands, entities, assets…"
          autocomplete="off"
          spellcheck="false"
        />
        <kbd class="omnibar-hint">Esc</kbd>
      </div>

      {#if results.length > 0}
        <ul class="omnibar-results" role="listbox">
          {#each results as result, i}
            <li
              class="omnibar-result"
              class:selected={i === selectedIndex}
              role="option"
              aria-selected={i === selectedIndex}
              onmouseenter={() => selectedIndex = i}
              onclick={() => execute(result)}
            >
              <span class="result-label">{resultLabel(result)}</span>
              <span class="result-meta">{resultMeta(result)}</span>
              {#if resultKeybind(result)}
                <kbd class="result-keybind">{resultKeybind(result)}</kbd>
              {/if}
            </li>
          {/each}
        </ul>
      {/if}
    </div>

    <!-- Backdrop to dismiss on outside click -->
    <div class="omnibar-backdrop" role="none" onclick={close}></div>
  {/if}
```

**New template (active state) — add `{#if query.trim() === ''}` branch for hint list:**
```svelte
  {:else}
    <!-- Active state -->
    <div class="omnibar-active">
      <div class="omnibar-input-row">
        <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor" class="omnibar-icon" aria-hidden="true">
          <circle cx="6" cy="6" r="4.5" stroke="currentColor" stroke-width="1.5" fill="none"/>
          <line x1="9.5" y1="9.5" x2="13" y2="13" stroke="currentColor" stroke-width="1.5"/>
        </svg>
        <input
          bind:this={inputEl}
          bind:value={query}
          type="text"
          class="omnibar-input"
          placeholder="Search commands, entities, assets…"
          autocomplete="off"
          spellcheck="false"
        />
        <kbd class="omnibar-hint">Esc</kbd>
      </div>

      {#if query.trim() === ''}
        <!-- Prefix hint list (shown when input is empty) -->
        <ul class="omnibar-results" role="listbox">
          {#each PREFIX_HINTS as hint}
            <li
              class="omnibar-result omnibar-prefix-hint"
              role="option"
              onclick={() => { query = hint.prefix; inputEl?.focus(); }}
            >
              <span class="hint-prefix">{hint.prefix}</span>
              <span class="result-label">{hint.label}</span>
              <span class="result-meta">{hint.description}</span>
            </li>
          {/each}
        </ul>
      {:else if results.length > 0}
        <ul class="omnibar-results" role="listbox">
          {#each results as result, i}
            <li
              class="omnibar-result"
              class:selected={i === selectedIndex}
              role="option"
              aria-selected={i === selectedIndex}
              onmouseenter={() => selectedIndex = i}
              onclick={() => execute(result)}
            >
              <span class="result-label">{resultLabel(result)}</span>
              <span class="result-meta">{resultMeta(result)}</span>
              {#if resultKeybind(result)}
                <kbd class="result-keybind">{resultKeybind(result)}</kbd>
              {/if}
            </li>
          {/each}
        </ul>
      {/if}
    </div>

    <!-- Backdrop to dismiss on outside click -->
    <div class="omnibar-backdrop" role="none" onclick={close}></div>
  {/if}
```

**Note on class naming:** The existing `<kbd class="omnibar-hint">` in the idle pill (line 174) and in the active input row (line 193) shares the `.omnibar-hint` CSS class. The prefix hint list items use a distinct class `omnibar-prefix-hint` (not `omnibar-hint`) to avoid colliding with the existing `<kbd>` styling rule.

### 5c — Add CSS for hint rows

**Where:** Append after the existing `.omnibar-backdrop` rule (after line 359) in the `<style>` block, before `</style>`.

```css
  .omnibar-prefix-hint {
    opacity: 0.75;
  }

  .omnibar-prefix-hint:hover {
    opacity: 1;
  }

  .hint-prefix {
    font-family: monospace;
    font-size: 13px;
    font-weight: 600;
    color: var(--color-accent, #89b4fa);
    min-width: 18px;
    text-align: center;
    flex-shrink: 0;
  }
```

**Steps:**
- [ ] Edit `engine/editor/src/lib/omnibar/Omnibar.svelte` `<script>`: add `PREFIX_HINTS` constant after the imports, before `interface Props`.
- [ ] Edit `engine/editor/src/lib/omnibar/Omnibar.svelte` template: replace the active-state `{:else}` branch as shown in 5b above.
- [ ] Edit `engine/editor/src/lib/omnibar/Omnibar.svelte` `<style>`: append `.omnibar-prefix-hint`, `.omnibar-prefix-hint:hover`, and `.hint-prefix` rules.
- [ ] Visual check: `npm run preview` from `engine/editor/` — open the omnibar with Ctrl+K and confirm the three hint rows (`>`, `@`, `#`) are visible. Click the `>` row and confirm the input value becomes `>` with focus retained.
- [ ] Run Playwright tests: `npm run test:e2e` from `engine/editor/`. Add these tests to `engine/editor/e2e/editor.spec.ts`:
  ```ts
  test('omnibar shows prefix hints when empty', async ({ page }) => {
    await page.keyboard.press('Control+k');
    const hints = page.locator('.omnibar-prefix-hint');
    await expect(hints).toHaveCount(3);
    await expect(hints.nth(0).locator('.hint-prefix')).toHaveText('>');
    await expect(hints.nth(1).locator('.hint-prefix')).toHaveText('@');
    await expect(hints.nth(2).locator('.hint-prefix')).toHaveText('#');
  });

  test('clicking hint types prefix into input', async ({ page }) => {
    await page.keyboard.press('Control+k');
    await page.locator('.omnibar-prefix-hint').first().click();
    const input = page.locator('.omnibar-input');
    await expect(input).toHaveValue('>');
    await expect(input).toBeFocused();
  });
  ```
- [ ] Commit: `feat(editor): show prefix hints in omnibar when input is empty`

---

## Task 6 — Add `showGroups` + `groupedResults` derived + section header render + CSS

**File:** `engine/editor/src/lib/omnibar/Omnibar.svelte`

### 6a — Add `showGroups` and `groupedResults` derived values in the `<script>` block

**Where:** After the `results` and `selectedIndex` state declarations (after line 29, before the `assets` state declaration).

**Add these two derived declarations:**
```ts
  const showGroups = $derived(
    new Set(results.map(r => r.group)).size > 1
  );

  const groupedResults = $derived((() => {
    const groups: { label: string; items: OmnibarResult[] }[] = [];
    const seen = new Map<string, OmnibarResult[]>();
    for (const r of results) {
      const key = r.group ?? '';
      if (!seen.has(key)) {
        seen.set(key, []);
        groups.push({ label: key, items: seen.get(key)! });
      }
      seen.get(key)!.push(r);
    }
    return groups;
  })());
```

**Design notes:**
- `showGroups` uses `new Set` to count distinct `group` values. When only one group is present (prefix-filtered results) it returns `false` and headers are suppressed.
- `groupedResults` preserves insertion order from `results[]`. Results with no `group` set (empty string key) are grouped together under an empty label — they render without a visible header text, which is acceptable for edge cases.
- Both are `$derived` (Svelte 5 runes) so they update reactively whenever `results` changes.

### 6b — Replace the results `{:else if results.length > 0}` branch in the template

**Where:** In the active-state template added in Task 5b, replace the `{:else if results.length > 0}` block (the normal results list) with the grouped/flat conditional render.

**Current (after Task 5b):**
```svelte
      {:else if results.length > 0}
        <ul class="omnibar-results" role="listbox">
          {#each results as result, i}
            <li
              class="omnibar-result"
              class:selected={i === selectedIndex}
              role="option"
              aria-selected={i === selectedIndex}
              onmouseenter={() => selectedIndex = i}
              onclick={() => execute(result)}
            >
              <span class="result-label">{resultLabel(result)}</span>
              <span class="result-meta">{resultMeta(result)}</span>
              {#if resultKeybind(result)}
                <kbd class="result-keybind">{resultKeybind(result)}</kbd>
              {/if}
            </li>
          {/each}
        </ul>
```

**New — grouped branch wraps `{#if showGroups}` around a `{#each groupedResults}` loop:**
```svelte
      {:else if results.length > 0}
        <ul class="omnibar-results" role="listbox">
          {#if showGroups}
            {#each groupedResults as group}
              {#if group.label}
                <li class="omnibar-section-header" role="presentation">{group.label}</li>
              {/if}
              {#each group.items as result}
                {@const i = results.indexOf(result)}
                <li
                  class="omnibar-result"
                  class:selected={i === selectedIndex}
                  role="option"
                  aria-selected={i === selectedIndex}
                  onmouseenter={() => selectedIndex = i}
                  onclick={() => execute(result)}
                >
                  <span class="result-label">{resultLabel(result)}</span>
                  <span class="result-meta">{resultMeta(result)}</span>
                  {#if resultKeybind(result)}
                    <kbd class="result-keybind">{resultKeybind(result)}</kbd>
                  {/if}
                </li>
              {/each}
            {/each}
          {:else}
            {#each results as result, i}
              <li
                class="omnibar-result"
                class:selected={i === selectedIndex}
                role="option"
                aria-selected={i === selectedIndex}
                onmouseenter={() => selectedIndex = i}
                onclick={() => execute(result)}
              >
                <span class="result-label">{resultLabel(result)}</span>
                <span class="result-meta">{resultMeta(result)}</span>
                {#if resultKeybind(result)}
                  <kbd class="result-keybind">{resultKeybind(result)}</kbd>
                {/if}
              </li>
            {/each}
          {/if}
        </ul>
```

**Keyboard navigation note:** `selectedIndex` always indexes into the flat `results[]` array — this is unchanged from the existing `onKeydown` handler (lines 122–139). Section headers (`li.omnibar-section-header`) are render-only; they are never part of `results[]`, never entered into the index space, and arrow key navigation skips over them naturally because the index arithmetic operates solely on `results[]`. The `results.indexOf(result)` lookup in the grouped branch recovers the correct flat index for each item without any DOM inspection.

### 6c — Add CSS for section headers

**Where:** Append after the `.hint-prefix` rule added in Task 5c, before `</style>`.

```css
  .omnibar-section-header {
    padding: 4px 10px 2px;
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--color-textDim, #555);
    user-select: none;
    pointer-events: none;
  }
```

**Steps:**
- [ ] Edit `engine/editor/src/lib/omnibar/Omnibar.svelte` `<script>`: add `showGroups` and `groupedResults` derived declarations after the `selectedIndex` state declaration (line 29).
- [ ] Edit `engine/editor/src/lib/omnibar/Omnibar.svelte` template: replace the `{:else if results.length > 0}` results block with the grouped/flat conditional as shown in 6b above.
- [ ] Edit `engine/editor/src/lib/omnibar/Omnibar.svelte` `<style>`: append the `.omnibar-section-header` rule.
- [ ] Visual check: `npm run preview` from `engine/editor/` — type a query that returns mixed results (e.g. a word that matches both a command label and an entity name). Confirm section headers appear between groups. Type `>` prefix — confirm no section headers appear (single-group mode).
- [ ] Verify keyboard navigation: open omnibar, type a mixed query, use arrow keys — confirm the highlight moves through result rows only, jumping visually over any section header rows.
- [ ] Run Playwright tests: `npm run test:e2e` from `engine/editor/`.
- [ ] Commit: `feat(editor): add grouped results with section headers to omnibar`

---

## Final Verification

- [ ] `npm run test` from `engine/editor/` — all unit tests green.
- [ ] `npm run test:e2e` from `engine/editor/` — all Playwright tests green including the four new tests added across Tasks 4 and 5.
- [ ] `npm run preview` from `engine/editor/` — manual walkthrough: Ctrl+K opens below titlebar, empty state shows three prefix hints, clicking a hint types the prefix, typing a mixed query shows grouped results with section headers, typing `>` shows flat command list with no headers, Escape closes cleanly.
