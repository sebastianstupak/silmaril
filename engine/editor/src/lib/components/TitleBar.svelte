<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { dispatchCommand } from '$lib/dispatch';
  import { aiServerRunning, startAiServer, stopAiServer } from '$lib/stores/ai-server';
  import type { SavedLayout } from '../docking/store';
  import type { EditorLayout } from '../docking/types';
  import { buildMinimap, buildIcon } from '../docking/minimap';
  import { formatKeybindDisplay } from '../keybind-utils';
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
  import { t } from '$lib/i18n';
  import Omnibar from '$lib/omnibar/Omnibar.svelte';
  import type { RecentItem } from '$lib/stores/recent-items';
  import type { PanelContribution } from '$lib/contributions/registry';

  // ── Props ──────────────────────────────────────────────────────────────────
  interface Props {
    savedLayouts?: SavedLayout[];
    activeLayoutId?: string | null;
    isDirty?: boolean;
    activePanels?: Set<string>;
    onApplyLayout?: (id: string) => void;
    onSaveToSlot?: (id: string) => void;
    onResetSlot?: (id: string) => void;
    onRenameSlot?: (id: string, name: string) => void;
    onDuplicateSlot?: (id: string) => void;
    onDeleteSlot?: (id: string) => void;
    onCreateLayout?: (name: string) => void;
    onAddPanel?: (panelId: string) => void;
    onSettingsOpen?: () => void;
    onOpenProject?: () => void;
    onLayoutReset?: () => void;
    onUndo?: () => void;
    onRedo?: () => void;
    canUndo?: boolean;
    canRedo?: boolean;
    compactMenu?: boolean;
    omnibarOpen?: boolean;
    onOmnibarOpen?: () => void;
    onOmnibarClose?: () => void;
    projectPath?: string | null;
    recentItems?: RecentItem[];
    panelContributions?: PanelContribution[];
  }

  let {
    savedLayouts = [],
    activeLayoutId = null,
    isDirty = false,
    activePanels = new Set(),
    onApplyLayout,
    onSaveToSlot,
    onResetSlot,
    onRenameSlot,
    onDuplicateSlot,
    onDeleteSlot,
    onCreateLayout,
    onAddPanel,
    onSettingsOpen,
    onOpenProject,
    onLayoutReset,
    onUndo,
    onRedo,
    canUndo = false,
    canRedo = false,
    compactMenu = false,
    omnibarOpen = false,
    onOmnibarOpen,
    onOmnibarClose,
    projectPath = null,
    recentItems = [],
    panelContributions = [],
  }: Props = $props();

  const MAX_VISIBLE_SLOTS = 4;

  let visibleSlots = $derived(savedLayouts.slice(0, MAX_VISIBLE_SLOTS));
  let overflowSlots = $derived(savedLayouts.slice(MAX_VISIBLE_SLOTS));

  // ── Hover card state ───────────────────────────────────────────────────────
  let hoveredSlot: string | null = $state(null);
  let hoverTimer: ReturnType<typeof setTimeout> | null = null;

  function onSlotEnter(id: string) {
    if (hoverTimer) clearTimeout(hoverTimer);
    hoverTimer = setTimeout(() => { hoveredSlot = id; }, 350);
  }
  function onSlotLeave() {
    if (hoverTimer) { clearTimeout(hoverTimer); hoverTimer = null; }
    hoveredSlot = null;
  }

  // ── Context menu state ─────────────────────────────────────────────────────
  let contextMenu: { slotId: string; x: number; y: number } | null = $state(null);

  function openContextMenu(e: MouseEvent, slotId: string) {
    e.preventDefault();
    e.stopPropagation();
    hoveredSlot = null;
    contextMenu = { slotId, x: e.clientX, y: e.clientY };
  }

  $effect(() => {
    if (!contextMenu) return;
    function close() { contextMenu = null; }
    const id = setTimeout(() => document.addEventListener('click', close, { once: true }), 0);
    return () => { clearTimeout(id); document.removeEventListener('click', close); };
  });

  // ── Rename state ───────────────────────────────────────────────────────────
  let renamingSlot: string | null = $state(null);
  let renameValue = $state('');
  let renameInput: HTMLInputElement | undefined = $state(undefined);

  $effect(() => {
    if (renamingSlot && renameInput) {
      renameInput.focus();
      renameInput.select();
    }
  });

  function startRename(slotId: string) {
    contextMenu = null;
    const slot = savedLayouts.find(s => s.id === slotId);
    if (!slot) return;
    renameValue = slot.name;
    renamingSlot = slotId;
  }

  function commitRename() {
    if (!renamingSlot) return;
    const name = renameValue.trim();
    if (name) onRenameSlot?.(renamingSlot, name);
    renamingSlot = null;
  }

  function cancelRename() {
    renamingSlot = null;
  }

  // ── Overflow dropdown ──────────────────────────────────────────────────────
  let showOverflow = $state(false);
  let creatingLayout = $state(false);
  let newLayoutName = $state('');
  let newLayoutInput: HTMLInputElement | undefined = $state(undefined);

  $effect(() => {
    if (creatingLayout && newLayoutInput) {
      newLayoutInput.focus();
    }
  });

  $effect(() => {
    if (!showOverflow) return;
    function close() { showOverflow = false; creatingLayout = false; }
    const id = setTimeout(() => document.addEventListener('click', close, { once: true }), 0);
    return () => { clearTimeout(id); document.removeEventListener('click', close); };
  });

  function startCreateLayout(e: MouseEvent) {
    e.stopPropagation();
    newLayoutName = '';
    creatingLayout = true;
  }

  function commitCreateLayout() {
    const name = newLayoutName.trim();
    if (name) onCreateLayout?.(name);
    showOverflow = false;
    creatingLayout = false;
  }

  // ── Panels dropdown ────────────────────────────────────────────────────────
  let showPanelsMenu = $state(false);

  $effect(() => {
    if (!showPanelsMenu) return;
    function close(e: MouseEvent) {
      if (!(e.target as HTMLElement).closest('.panels-btn-wrapper')) showPanelsMenu = false;
    }
    const id = setTimeout(() => document.addEventListener('click', close, { once: true }), 0);
    return () => { clearTimeout(id); document.removeEventListener('click', close); };
  });

  // ── Drag / window controls ─────────────────────────────────────────────────
  function onTitlebarMousedown(e: MouseEvent) {
    if (e.button !== 0) return;
    if ((e.target as HTMLElement).closest(
      'button, input, .slot-wrapper, .overflow-wrapper, ' +
      '.panels-btn-wrapper, .omnibar-wrapper, ' +
      '.titlebar-menus, [data-dropdown-menu-content], ' +
      '[data-dropdown-menu-item], ' +
      '[data-dropdown-menu-trigger][data-state="open"]'
    )) return;
    invoke('window_start_drag').catch(() => {});
  }

  function onTitlebarDblclick(e: MouseEvent) {
    if ((e.target as HTMLElement).closest(
      'button, input, .slot-wrapper, .overflow-wrapper, ' +
      '.panels-btn-wrapper, .omnibar-wrapper, ' +
      '.titlebar-menus, [data-dropdown-menu-content], ' +
      '[data-dropdown-menu-item], ' +
      '[data-dropdown-menu-trigger][data-state="open"]'
    )) return;
    invoke('window_toggle_maximize').catch(() => {});
  }

  function minimize() { invoke('window_minimize').catch(() => {}); }
  function maximize() { invoke('window_toggle_maximize').catch(() => {}); }
  function close()    { invoke('window_close').catch(() => {}); }

  // buildMinimap, buildIcon, and formatKeybindDisplay are imported above
</script>

<div
  class="titlebar"
  onmousedown={onTitlebarMousedown}
  ondblclick={onTitlebarDblclick}
  role="none"
>
  <!-- Left: logo + menus -->
  <div class="titlebar-left">
    <!-- Logo sub-area (drag-through) -->
    <div class="titlebar-logo">
      <svg class="app-icon" width="16" height="16" viewBox="0 0 16 16" fill="none" aria-hidden="true">
        <polygon points="6,1 10,1 15,6 15,10 10,15 6,15 1,10 1,6" fill="none" stroke="var(--color-accent,#007acc)" stroke-width="1.2"/>
        <polygon points="6.5,4 9.5,4 12,6.5 12,9.5 9.5,12 6.5,12 4,9.5 4,6.5" fill="var(--color-accent,#007acc)" opacity="0.35"/>
      </svg>
      <span class="app-name">Silmaril</span>
    </div>

    <!-- Menus sub-area -->
    <div class="titlebar-menus" class:compact={compactMenu} role="menubar">

      <!-- File -->
      <DropdownMenu.Root>
        <DropdownMenu.Trigger class="menu-trigger" title={t('menu.file')}>
          <svg width="14" height="14" viewBox="0 0 14 14" fill="currentColor" aria-hidden="true">
            <path d="M1 3a1 1 0 011-1h3.3l1.7 1.7H12a1 1 0 011 1v6a1 1 0 01-1 1H2a1 1 0 01-1-1V3z"/>
          </svg>
          <span class="menu-label">{t('menu.file')}</span>
        </DropdownMenu.Trigger>
        <DropdownMenu.Content align="start" sideOffset={4} class="min-w-[200px]">
          <DropdownMenu.Item onclick={() => onOpenProject?.()}>{t('menu.file.open_project')}</DropdownMenu.Item>
          <DropdownMenu.Item>
            {t('menu.file.save_template')}
            <DropdownMenu.Shortcut>Ctrl+S</DropdownMenu.Shortcut>
          </DropdownMenu.Item>
          <DropdownMenu.Item>{t('menu.file.save_template_as')}</DropdownMenu.Item>
          <DropdownMenu.Item>{t('menu.file.new_template')}</DropdownMenu.Item>
          <DropdownMenu.Sub>
            <DropdownMenu.SubTrigger>{t('menu.file.recent_projects')}</DropdownMenu.SubTrigger>
            <DropdownMenu.SubContent>
              <DropdownMenu.Item disabled>{t('menu.file.no_recent_projects')}</DropdownMenu.Item>
            </DropdownMenu.SubContent>
          </DropdownMenu.Sub>
          <DropdownMenu.Separator />
          <DropdownMenu.Item onclick={() => invoke('window_close').catch(() => {})}>{t('menu.file.exit')}</DropdownMenu.Item>
        </DropdownMenu.Content>
      </DropdownMenu.Root>

      <!-- Edit -->
      <DropdownMenu.Root>
        <DropdownMenu.Trigger class="menu-trigger" title={t('menu.edit')}>
          <svg width="14" height="14" viewBox="0 0 14 14" fill="currentColor" aria-hidden="true">
            <path d="M9.5 1.5l3 3-7 7-3.5.5.5-3.5 7-7zm0-1a.5.5 0 01.35.15l3 3a.5.5 0 010 .7l-7 7a.5.5 0 01-.25.14L2.1 12a.5.5 0 01-.6-.6l.5-3.5a.5.5 0 01.14-.26l7-7A.5.5 0 019.5.5z"/>
          </svg>
          <span class="menu-label">{t('menu.edit')}</span>
        </DropdownMenu.Trigger>
        <DropdownMenu.Content align="start" sideOffset={4} class="min-w-[200px]">
          <DropdownMenu.Item onclick={() => dispatchCommand('edit.undo').catch(console.error)} disabled={!canUndo}>
            {t('menu.edit.undo')}
            <DropdownMenu.Shortcut>Ctrl+Z</DropdownMenu.Shortcut>
          </DropdownMenu.Item>
          <DropdownMenu.Item onclick={() => dispatchCommand('edit.redo').catch(console.error)} disabled={!canRedo}>
            {t('menu.edit.redo')}
            <DropdownMenu.Shortcut>Ctrl+Y</DropdownMenu.Shortcut>
          </DropdownMenu.Item>
          <DropdownMenu.Separator />
          <DropdownMenu.Item>
            {t('menu.edit.cut')}
            <DropdownMenu.Shortcut>Ctrl+X</DropdownMenu.Shortcut>
          </DropdownMenu.Item>
          <DropdownMenu.Item>
            {t('menu.edit.copy')}
            <DropdownMenu.Shortcut>Ctrl+C</DropdownMenu.Shortcut>
          </DropdownMenu.Item>
          <DropdownMenu.Item>
            {t('menu.edit.paste')}
            <DropdownMenu.Shortcut>Ctrl+V</DropdownMenu.Shortcut>
          </DropdownMenu.Item>
          <DropdownMenu.Separator />
          <DropdownMenu.Item>
            {t('menu.edit.duplicate')}
            <DropdownMenu.Shortcut>Ctrl+D</DropdownMenu.Shortcut>
          </DropdownMenu.Item>
          <DropdownMenu.Item>
            {t('menu.edit.delete')}
            <DropdownMenu.Shortcut>Del</DropdownMenu.Shortcut>
          </DropdownMenu.Item>
          <DropdownMenu.Item>
            {t('menu.edit.select_all')}
            <DropdownMenu.Shortcut>Ctrl+A</DropdownMenu.Shortcut>
          </DropdownMenu.Item>
          <DropdownMenu.Separator />
          <DropdownMenu.Item onclick={() => onSettingsOpen?.()}>
            {t('settings.title')}
            <DropdownMenu.Shortcut>Ctrl+,</DropdownMenu.Shortcut>
          </DropdownMenu.Item>
        </DropdownMenu.Content>
      </DropdownMenu.Root>

      <!-- Entity -->
      <DropdownMenu.Root>
        <DropdownMenu.Trigger class="menu-trigger" title={t('menu.entity')}>
          <svg width="14" height="14" viewBox="0 0 14 14" fill="currentColor" aria-hidden="true">
            <path d="M7 1L1 4.5v5L7 13l6-3.5v-5L7 1zm0 1.2l4.5 2.6v.7L7 8.1 2.5 5.5v-.7L7 2.2zM2 6.4l4.5 2.6v3.3L2 9.7V6.4zm5.5 5.9V8.9L12 6.4v3.3l-4.5 2.6z"/>
          </svg>
          <span class="menu-label">{t('menu.entity')}</span>
        </DropdownMenu.Trigger>
        <DropdownMenu.Content align="start" sideOffset={4} class="min-w-[200px]">
          <DropdownMenu.Item>{t('menu.entity.create_empty')}</DropdownMenu.Item>
          <DropdownMenu.Item>{t('menu.entity.create_from_template')}</DropdownMenu.Item>
          <DropdownMenu.Separator />
          <DropdownMenu.Item>{t('menu.entity.add_component')}</DropdownMenu.Item>
        </DropdownMenu.Content>
      </DropdownMenu.Root>

      <!-- Build -->
      <DropdownMenu.Root>
        <DropdownMenu.Trigger class="menu-trigger" title={t('menu.build')}>
          <svg width="14" height="14" viewBox="0 0 14 14" fill="currentColor" aria-hidden="true">
            <path d="M9.5 1a3 3 0 00-2.83 4L1.5 10.2a1 1 0 000 1.4l.9.9a1 1 0 001.4 0L9 7.33A3 3 0 109.5 1zm0 4.5a1.5 1.5 0 110-3 1.5 1.5 0 010 3z"/>
          </svg>
          <span class="menu-label">{t('menu.build')}</span>
        </DropdownMenu.Trigger>
        <DropdownMenu.Content align="start" sideOffset={4} class="min-w-[200px]">
          <DropdownMenu.Item>
            {t('menu.build.build_project')}
            <DropdownMenu.Shortcut>Ctrl+B</DropdownMenu.Shortcut>
          </DropdownMenu.Item>
          <DropdownMenu.Item>{t('menu.build.build_release')}</DropdownMenu.Item>
          <DropdownMenu.Separator />
          <DropdownMenu.Item>{t('menu.build.package')}</DropdownMenu.Item>
          <DropdownMenu.Item>{t('menu.build.platform_settings')}</DropdownMenu.Item>
        </DropdownMenu.Content>
      </DropdownMenu.Root>

      <!-- Modules -->
      <DropdownMenu.Root>
        <DropdownMenu.Trigger class="menu-trigger" title={t('menu.modules')}>
          <svg width="14" height="14" viewBox="0 0 14 14" fill="currentColor" aria-hidden="true">
            <path d="M5.5 1A1.5 1.5 0 004 2.5v.5H2.5A1.5 1.5 0 001 4.5v7A1.5 1.5 0 002.5 13h9a1.5 1.5 0 001.5-1.5v-7A1.5 1.5 0 0011.5 3H10v-.5A1.5 1.5 0 008.5 1h-3zM5 2.5a.5.5 0 01.5-.5h3a.5.5 0 01.5.5V3H5v-.5zM3.5 5h1a.5.5 0 010 1h-1a.5.5 0 010-1zm3 0h4a.5.5 0 010 1h-4a.5.5 0 010-1zm-3 3h1a.5.5 0 010 1h-1a.5.5 0 010-1zm3 0h4a.5.5 0 010 1h-4a.5.5 0 010-1z"/>
          </svg>
          <span class="menu-label">{t('menu.modules')}</span>
        </DropdownMenu.Trigger>
        <DropdownMenu.Content align="start" sideOffset={4} class="min-w-[200px]">
          <DropdownMenu.Item>{t('menu.modules.add_module')}</DropdownMenu.Item>
          <DropdownMenu.Item>{t('menu.modules.manage_modules')}</DropdownMenu.Item>
        </DropdownMenu.Content>
      </DropdownMenu.Root>

      <!-- Help -->
      <DropdownMenu.Root>
        <DropdownMenu.Trigger class="menu-trigger" title={t('menu.help')}>
          <svg width="14" height="14" viewBox="0 0 14 14" fill="currentColor" aria-hidden="true">
            <path d="M7 1a6 6 0 100 12A6 6 0 007 1zm0 9.5a.75.75 0 110-1.5.75.75 0 010 1.5zm.75-3.5a.75.75 0 01-1.5 0V7c0-.41.34-.75.75-.75a1.25 1.25 0 000-2.5A1.25 1.25 0 005.75 5a.75.75 0 01-1.5 0 2.75 2.75 0 115.5 0c0 1.07-.62 2-1.5 2.45V7z"/>
          </svg>
          <span class="menu-label">{t('menu.help')}</span>
        </DropdownMenu.Trigger>
        <DropdownMenu.Content align="start" sideOffset={4} class="min-w-[200px]">
          <DropdownMenu.Item>{t('menu.help.documentation')}</DropdownMenu.Item>
          <DropdownMenu.Separator />
          <DropdownMenu.Item>{t('menu.help.about')}</DropdownMenu.Item>
        </DropdownMenu.Content>
      </DropdownMenu.Root>

    </div>
  </div>

  <!-- Center: omnibar -->
  <div class="titlebar-center">
    <Omnibar
      bind:open={omnibarOpen}
      {projectPath}
      {recentItems}
      onOpen={onOmnibarOpen}
      onClose={onOmnibarClose}
    />
  </div>

  <!-- Right: layout slots · overflow · panels · window controls -->
  <div class="titlebar-right">

    <!-- ── Layout slot buttons ── -->
    {#each visibleSlots as slot}
      {@const isActive = slot.id === activeLayoutId}
      {@const isDirtySlot = isActive && isDirty}
      <div
        class="slot-wrapper"
        onmouseenter={() => onSlotEnter(slot.id)}
        onmouseleave={onSlotLeave}
        role="none"
      >
        {#if renamingSlot === slot.id}
          <input
            class="slot-rename"
            bind:value={renameValue}
            bind:this={renameInput}
            onkeydown={(e) => { if (e.key === 'Enter') commitRename(); if (e.key === 'Escape') cancelRename(); }}
            onblur={commitRename}
            onclick={(e) => e.stopPropagation()}
          />
        {:else}
          <button
            class="slot-btn"
            class:active={isActive}
            class:dirty={isDirtySlot}
            onclick={() => onApplyLayout?.(slot.id)}
            oncontextmenu={(e) => openContextMenu(e, slot.id)}
            title={slot.keybind ? formatKeybindDisplay(slot.keybind) : slot.name}
            aria-pressed={isActive}
          >
            <span class="slot-icon" aria-hidden="true">{@html buildIcon(slot.layout, 16, 11)}</span>
            <span class="slot-name">{slot.name}</span>
            {#if isDirtySlot}
              <!-- Dirty dot — click to save -->
              <span
                class="dirty-dot"
                title="Unsaved changes — click to save"
                onclick={(e) => { e.stopPropagation(); onSaveToSlot?.(slot.id); }}
                role="button"
                tabindex={-1}
                aria-label="Save layout changes"
              ></span>
            {/if}
          </button>
        {/if}

        <!-- Hover card -->
        {#if hoveredSlot === slot.id && renamingSlot !== slot.id}
          <div class="slot-hover-card" role="tooltip">
            <div class="hover-card-header">
              <span class="hover-card-name">{slot.name}</span>
              {#if slot.keybind}
                <span class="hover-card-keybind">{formatKeybindDisplay(slot.keybind)}</span>
              {/if}
            </div>
            <div class="hover-card-minimap">
              {@html buildMinimap(slot.layout)}
            </div>
          </div>
        {/if}
      </div>
    {/each}

    <!-- ── Overflow dropdown (5th+ slots) ── -->
    {#if overflowSlots.length > 0 || true}
      <div class="overflow-wrapper">
        <button
          class="slot-btn overflow-btn"
          onclick={(e) => { e.stopPropagation(); showOverflow = !showOverflow; }}
          aria-expanded={showOverflow}
          aria-haspopup="menu"
          title="More layouts"
        >
          {#if overflowSlots.length > 0}
            <span class="overflow-count">+{overflowSlots.length}</span>
          {:else}
            <svg width="12" height="12" viewBox="0 0 12 12" fill="currentColor" aria-hidden="true">
              <circle cx="2" cy="6" r="1.2"/><circle cx="6" cy="6" r="1.2"/><circle cx="10" cy="6" r="1.2"/>
            </svg>
          {/if}
        </button>

        {#if showOverflow}
          <div class="overflow-menu" role="menu">
            {#if overflowSlots.length > 0}
              <div class="overflow-section-label">More layouts</div>
              {#each overflowSlots as slot}
                {@const isActive = slot.id === activeLayoutId}
                <button
                  class="overflow-item"
                  class:active={isActive}
                  onclick={() => { onApplyLayout?.(slot.id); showOverflow = false; }}
                  oncontextmenu={(e) => openContextMenu(e, slot.id)}
                  role="menuitem"
                >
                  <span class="overflow-item-name">{slot.name}</span>
                  {#if slot.keybind}
                    <span class="overflow-item-keybind">{formatKeybindDisplay(slot.keybind)}</span>
                  {/if}
                </button>
              {/each}
              <div class="overflow-divider"></div>
            {/if}

            {#if creatingLayout}
              <div class="overflow-create-row" onclick={(e) => e.stopPropagation()} role="none">
                <input
                  class="overflow-name-input"
                  bind:value={newLayoutName}
                  bind:this={newLayoutInput}
                  placeholder="Layout name…"
                  onkeydown={(e) => { if (e.key === 'Enter') commitCreateLayout(); if (e.key === 'Escape') { creatingLayout = false; } }}
                  onblur={() => { if (!newLayoutName.trim()) creatingLayout = false; }}
                />
                <button class="overflow-create-confirm" onclick={commitCreateLayout} title="Save">
                  <svg width="10" height="10" viewBox="0 0 10 10" fill="none">
                    <path d="M1 5l2.5 2.5 5.5-5.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
                  </svg>
                </button>
              </div>
            {:else}
              <button class="overflow-item overflow-create" onclick={startCreateLayout} role="menuitem">
                <svg width="10" height="10" viewBox="0 0 10 10" fill="currentColor" aria-hidden="true">
                  <rect x="4" y="1" width="2" height="8" rx="0.5"/>
                  <rect x="1" y="4" width="8" height="2" rx="0.5"/>
                </svg>
                <span>Save current as new layout</span>
              </button>
            {/if}
          </div>
        {/if}
      </div>
    {/if}

    <!-- ── AI Server indicator ── -->
    <button
      class="icon-btn ai-server-btn"
      class:ai-running={$aiServerRunning}
      onclick={() => $aiServerRunning ? stopAiServer() : startAiServer('')}
      title={$aiServerRunning ? 'AI Server running — click to stop' : 'Start AI Server'}
      aria-label={$aiServerRunning ? 'Stop AI Server' : 'Start AI Server'}
    >
      <svg width="12" height="14" viewBox="0 0 12 14" fill="currentColor" aria-hidden="true">
        <path d="M7 1L1 8h5l-1 5 6-7H6l1-5z"/>
      </svg>
      <span class="ai-server-dot" aria-hidden="true"></span>
    </button>

    <!-- ── Panel management ── -->
    <div class="panels-btn-wrapper">
      <button
        class="icon-btn"
        onclick={() => showPanelsMenu = !showPanelsMenu}
        title="Manage panels"
        aria-label="Manage panels"
        aria-expanded={showPanelsMenu}
      >
        <svg width="14" height="12" viewBox="0 0 14 12" fill="currentColor" aria-hidden="true">
          <rect x="0" y="0" width="6" height="5" rx="0.5" opacity="0.75"/>
          <rect x="8" y="0" width="6" height="5" rx="0.5" opacity="0.75"/>
          <rect x="0" y="7" width="6" height="5" rx="0.5" opacity="0.5"/>
          <rect x="8" y="7" width="6" height="5" rx="0.5" opacity="0.5"/>
        </svg>
      </button>

      {#if showPanelsMenu}
        <div class="panels-menu" role="menu" aria-label="Panels">
          <div class="panels-menu-header">Panels</div>
          {#each panelContributions as panel}
            {@const isOpen = activePanels.has(panel.id)}
            <button
              class="panel-item"
              class:is-open={isOpen}
              onclick={(e) => { e.stopPropagation(); if (!isOpen) { onAddPanel?.(panel.id); } showPanelsMenu = false; }}
              role="menuitem"
              title={isOpen ? `${panel.title} is already open` : `Add ${panel.title}`}
            >
              <span class="panel-indicator" aria-hidden="true">
                {#if isOpen}
                  <svg width="10" height="8" viewBox="0 0 10 8" fill="none">
                    <path d="M1 4l2.5 2.5 5.5-6" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
                  </svg>
                {:else}
                  <svg width="10" height="10" viewBox="0 0 10 10" fill="currentColor">
                    <rect x="4" y="1" width="2" height="8" rx="0.5"/>
                    <rect x="1" y="4" width="8" height="2" rx="0.5"/>
                  </svg>
                {/if}
              </span>
              <span>{panel.title}</span>
            </button>
          {/each}
        </div>
      {/if}
    </div>

    <!-- ── Settings shortcut ── -->
    <button
      class="icon-btn settings-btn"
      onclick={() => onSettingsOpen?.()}
      title="{t('settings.title')} (Ctrl+,)"
      aria-label={t('settings.title')}
    >
      <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
        <path d="M8 4.754a3.246 3.246 0 1 0 0 6.492 3.246 3.246 0 0 0 0-6.492zM5.754 8a2.246 2.246 0 1 1 4.492 0 2.246 2.246 0 0 1-4.492 0z"/>
        <path d="M9.796 1.343c-.527-1.79-3.065-1.79-3.592 0l-.094.319a.873.873 0 0 1-1.255.52l-.292-.16c-1.64-.892-3.433.902-2.54 2.541l.159.292a.873.873 0 0 1-.52 1.255l-.319.094c-1.79.527-1.79 3.065 0 3.592l.319.094a.873.873 0 0 1 .52 1.255l-.16.292c-.892 1.64.901 3.434 2.541 2.54l.292-.159a.873.873 0 0 1 1.255.52l.094.319c.527 1.79 3.065 1.79 3.592 0l.094-.319a.873.873 0 0 1 1.255-.52l.292.16c1.64.893 3.434-.902 2.54-2.541l-.159-.292a.873.873 0 0 1 .52-1.255l.319-.094c1.79-.527 1.79-3.065 0-3.592l-.319-.094a.873.873 0 0 1-.52-1.255l.16-.292c.893-1.64-.902-3.433-2.541-2.54l-.292.159a.873.873 0 0 1-1.255-.52l-.094-.319z"/>
      </svg>
    </button>

    <!-- ── Window controls ── -->
    <div class="window-controls" aria-label="Window controls">
      <button class="wc-btn wc-minimize" onclick={minimize} title="Minimize" aria-label="Minimize">
        <svg width="10" height="1" viewBox="0 0 10 1" aria-hidden="true"><rect width="10" height="1" fill="currentColor"/></svg>
      </button>
      <button class="wc-btn wc-maximize" onclick={maximize} title="Maximize / Restore" aria-label="Maximize">
        <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
          <rect x="0.5" y="0.5" width="9" height="9" fill="none" stroke="currentColor" stroke-width="1"/>
        </svg>
      </button>
      <button class="wc-btn wc-close" onclick={close} title="Close" aria-label="Close">
        <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
          <line x1="0" y1="0" x2="10" y2="10" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
          <line x1="10" y1="0" x2="0" y2="10" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
        </svg>
      </button>
    </div>
  </div>
</div>

<!-- Context menu (fixed, outside normal flow) -->
{#if contextMenu}
  {@const slot = savedLayouts.find(s => s.id === contextMenu?.slotId)}
  {@const isActiveSlot = contextMenu.slotId === activeLayoutId}
  <div
    class="context-menu"
    style="top: {contextMenu.y}px; left: {contextMenu.x}px"
    onclick={(e) => e.stopPropagation()}
    role="menu"
    aria-label="Layout options"
  >
    {#if isActiveSlot && isDirty}
      <button class="ctx-item ctx-save" onclick={() => { onSaveToSlot?.(contextMenu!.slotId); contextMenu = null; }} role="menuitem">
        <svg width="12" height="12" viewBox="0 0 12 12" fill="none" aria-hidden="true">
          <path d="M1.5 6l3 3 6-6" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
        Save changes
      </button>
      <div class="ctx-divider"></div>
    {/if}
    <button class="ctx-item" onclick={() => { onResetSlot?.(contextMenu!.slotId); contextMenu = null; }} role="menuitem">Reset to saved</button>
    <button class="ctx-item" onclick={() => startRename(contextMenu!.slotId)} role="menuitem">Rename</button>
    <div class="ctx-divider"></div>
    <button class="ctx-item" onclick={() => { onDuplicateSlot?.(contextMenu!.slotId); contextMenu = null; }} role="menuitem">Duplicate</button>
    {#if savedLayouts.length > 1}
      <button class="ctx-item ctx-danger" onclick={() => { onDeleteSlot?.(contextMenu!.slotId); contextMenu = null; }} role="menuitem">Delete</button>
    {/if}
  </div>
{/if}

<style>
  /* ── Titlebar shell ─────────────────────────────────────────────────────── */
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
    position: relative;
    z-index: 50;
  }

  /* ── Left ─────────────────────────────────────────────────────────────────── */
  .titlebar-left {
    display: flex;
    align-items: center;
    min-width: 0;
    justify-self: start;
  }

  /* Logo sub-area: drag-through, non-interactive */
  .titlebar-logo {
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 0 10px 0 14px;
    flex-shrink: 0;
    pointer-events: none;
  }

  /* Menus sub-area */
  .titlebar-menus {
    display: flex;
    align-items: center;
    gap: 0;
    pointer-events: auto;
  }

  .titlebar-menus :global(.menu-trigger) {
    all: unset;
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 0 10px;
    font-size: 12px;
    font-weight: 500;
    color: var(--color-textMuted, #999);
    cursor: default;
    user-select: none;
    border-radius: 3px;
    height: 26px;
    white-space: nowrap;
    min-width: 28px;
    box-sizing: border-box;
  }

  .titlebar-menus :global(.menu-trigger:hover),
  .titlebar-menus :global(.menu-trigger[data-state="open"]) {
    background: rgba(255, 255, 255, 0.08);
    color: var(--color-textBright, #fff);
  }

  /* Compact mode: icon only */
  .titlebar-menus.compact :global(.menu-label) {
    display: none;
  }
  .app-icon { flex-shrink: 0; }
  .app-name {
    font-size: 12px;
    font-weight: 500;
    color: var(--color-textDim, #666);
    letter-spacing: 0.03em;
  }

  /* ── Center ─────────────────────────────────────────────────────────────── */
  .titlebar-center {
    display: flex;
    align-items: center;
    justify-content: center;
    min-width: 0;
  }

  /* ── Right ──────────────────────────────────────────────────────────────── */
  .titlebar-right {
    display: flex;
    align-items: center;
    align-self: stretch;
    justify-self: end;
    gap: 1px;
    padding-right: 0;
  }

  /* ── Slot buttons ────────────────────────────────────────────────────────── */
  .slot-wrapper {
    position: relative;
    display: flex;
    align-items: center;
  }

  .slot-btn {
    height: 22px;
    padding: 0 9px;
    display: flex;
    align-items: center;
    gap: 5px;
    background: none;
    border: 1px solid transparent;
    border-radius: 3px;
    color: var(--color-textDim, #666);
    font-size: 11px;
    font-weight: 500;
    cursor: pointer;
    transition: color 0.1s, background 0.1s, border-color 0.1s;
    white-space: nowrap;
  }
  .slot-btn:hover {
    color: var(--color-textMuted, #999);
    background: rgba(255,255,255,0.05);
    border-color: color-mix(in srgb, var(--color-border, #404040) 50%, transparent);
  }
  .slot-btn.active {
    color: var(--color-text, #ccc);
    background: rgba(255,255,255,0.07);
    border-color: color-mix(in srgb, var(--color-border, #404040) 80%, transparent);
  }
  .slot-icon {
    display: flex;
    align-items: center;
    flex-shrink: 0;
    opacity: 0.5;
    pointer-events: none;
  }
  .slot-btn:hover .slot-icon { opacity: 0.7; }
  .slot-btn.active .slot-icon { opacity: 0.9; }
  .slot-icon :global(svg) { display: block; }

  .slot-name { pointer-events: none; }

  /* ── Dirty dot ───────────────────────────────────────────────────────────── */
  .dirty-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--color-accent, #007acc);
    opacity: 0.9;
    flex-shrink: 0;
    cursor: pointer;
    transition: transform 0.1s, opacity 0.1s;
  }
  .dirty-dot:hover {
    transform: scale(1.3);
    opacity: 1;
  }

  /* ── Rename input ────────────────────────────────────────────────────────── */
  .slot-rename {
    height: 22px;
    padding: 0 6px;
    background: var(--color-bgPanel, #252525);
    border: 1px solid var(--color-accent, #007acc);
    border-radius: 3px;
    color: var(--color-text, #ccc);
    font-size: 11px;
    font-weight: 500;
    outline: none;
    width: 90px;
  }

  /* ── Hover card ──────────────────────────────────────────────────────────── */
  .slot-hover-card {
    position: absolute;
    top: calc(100% + 8px);
    left: 50%;
    transform: translateX(-50%);
    z-index: 200;
    background: var(--color-bgPanel, #252525);
    border: 1px solid var(--color-border, #404040);
    border-radius: 6px;
    padding: 8px;
    box-shadow: 0 8px 24px rgba(0,0,0,0.5);
    pointer-events: none;
    min-width: 140px;
  }
  .hover-card-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 8px;
  }
  .hover-card-name {
    font-size: 11px;
    font-weight: 600;
    color: var(--color-text, #ccc);
  }
  .hover-card-keybind {
    font-size: 10px;
    color: var(--color-textDim, #666);
    background: rgba(255,255,255,0.07);
    border: 1px solid color-mix(in srgb, var(--color-border, #404040) 60%, transparent);
    border-radius: 3px;
    padding: 1px 5px;
    white-space: nowrap;
    flex-shrink: 0;
  }
  .hover-card-minimap {
    color: var(--color-text, #ccc);
    line-height: 0;
  }
  .hover-card-minimap :global(svg) {
    display: block;
  }

  /* ── Overflow button & dropdown ──────────────────────────────────────────── */
  .overflow-wrapper {
    position: relative;
    display: flex;
    align-items: center;
  }
  .overflow-btn {
    padding: 0 7px;
    font-size: 10px;
  }
  .overflow-count {
    font-size: 10px;
    font-weight: 600;
    opacity: 0.7;
  }
  .overflow-menu {
    position: absolute;
    top: calc(100% + 6px);
    right: 0;
    z-index: 200;
    background: var(--color-bgPanel, #252525);
    border: 1px solid var(--color-border, #404040);
    border-radius: 6px;
    padding: 4px;
    min-width: 200px;
    box-shadow: 0 8px 24px rgba(0,0,0,0.5);
  }
  .overflow-section-label {
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.07em;
    color: var(--color-textDim, #666);
    padding: 4px 8px 6px;
  }
  .overflow-item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 5px 8px;
    background: none;
    border: none;
    border-radius: 4px;
    color: var(--color-textMuted, #999);
    font-size: 12px;
    cursor: pointer;
    text-align: left;
    transition: background 0.1s;
  }
  .overflow-item:hover { background: rgba(255,255,255,0.07); color: var(--color-text, #ccc); }
  .overflow-item.active { color: var(--color-text, #ccc); }
  .overflow-item-name { flex: 1; }
  .overflow-item-keybind {
    font-size: 10px;
    color: var(--color-textDim, #666);
    background: rgba(255,255,255,0.06);
    border: 1px solid color-mix(in srgb, var(--color-border, #404040) 60%, transparent);
    border-radius: 3px;
    padding: 1px 5px;
    flex-shrink: 0;
  }
  .overflow-divider {
    height: 1px;
    background: color-mix(in srgb, var(--color-border, #404040) 50%, transparent);
    margin: 3px 0;
  }
  .overflow-create {
    color: var(--color-textDim, #666);
  }
  .overflow-create-row {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 3px 6px;
  }
  .overflow-name-input {
    flex: 1;
    height: 24px;
    padding: 0 6px;
    background: rgba(255,255,255,0.05);
    border: 1px solid var(--color-accent, #007acc);
    border-radius: 3px;
    color: var(--color-text, #ccc);
    font-size: 12px;
    outline: none;
  }
  .overflow-create-confirm {
    width: 24px;
    height: 24px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(0,122,204,0.15);
    border: 1px solid rgba(0,122,204,0.4);
    border-radius: 3px;
    color: var(--color-accent, #007acc);
    cursor: pointer;
    flex-shrink: 0;
    transition: background 0.1s;
  }
  .overflow-create-confirm:hover { background: rgba(0,122,204,0.25); }

  /* ── Panels button ───────────────────────────────────────────────────────── */
  .panels-btn-wrapper {
    position: relative;
    display: flex;
    align-items: center;
    padding: 0 4px;
  }
  .icon-btn {
    width: 28px;
    height: 22px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: none;
    border: 1px solid transparent;
    border-radius: 3px;
    color: var(--color-textDim, #666);
    cursor: pointer;
    transition: color 0.1s, background 0.1s, border-color 0.1s;
    padding: 0;
  }
  .icon-btn:hover {
    color: var(--color-textMuted, #999);
    background: rgba(255,255,255,0.06);
    border-color: color-mix(in srgb, var(--color-border, #404040) 60%, transparent);
  }
  .panels-menu {
    position: absolute;
    top: calc(100% + 6px);
    right: 0;
    z-index: 200;
    background: var(--color-bgPanel, #252525);
    border: 1px solid var(--color-border, #404040);
    border-radius: 6px;
    padding: 4px;
    min-width: 160px;
    box-shadow: 0 8px 24px rgba(0,0,0,0.5);
  }
  .panels-menu-header {
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.07em;
    color: var(--color-textDim, #666);
    padding: 4px 8px 6px;
  }
  .panel-item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 5px 8px;
    background: none;
    border: none;
    border-radius: 4px;
    color: var(--color-text, #ccc);
    font-size: 12px;
    cursor: pointer;
    text-align: left;
    transition: background 0.1s;
  }
  .panel-item:hover { background: rgba(255,255,255,0.07); }
  .panel-item.is-open { color: var(--color-textMuted, #999); cursor: default; }
  .panel-item.is-open:hover { background: none; }
  .panel-indicator {
    width: 16px;
    height: 16px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    color: var(--color-textDim, #666);
  }
  .panel-item.is-open .panel-indicator { color: var(--color-accent, #007acc); }

  /* ── Window controls ─────────────────────────────────────────────────────── */
  .window-controls { display: flex; align-items: stretch; height: 100%; }
  .wc-btn {
    width: 46px;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    background: none;
    border: none;
    color: var(--color-textDim, #666);
    cursor: default;
    transition: background 0.1s, color 0.1s;
    padding: 0;
  }
  .wc-btn:hover { background: rgba(255,255,255,0.08); color: var(--color-textMuted, #999); }
  .wc-close:hover { background: #c42b1c; color: #fff; }

  /* ── Context menu ────────────────────────────────────────────────────────── */
  .context-menu {
    position: fixed;
    z-index: 9999;
    background: var(--color-bgPanel, #252525);
    border: 1px solid var(--color-border, #404040);
    border-radius: 6px;
    padding: 4px;
    min-width: 160px;
    box-shadow: 0 8px 24px rgba(0,0,0,0.5);
  }
  .ctx-item {
    display: flex;
    align-items: center;
    gap: 7px;
    width: 100%;
    padding: 5px 8px;
    background: none;
    border: none;
    border-radius: 4px;
    color: var(--color-textMuted, #999);
    font-size: 12px;
    cursor: pointer;
    text-align: left;
    transition: background 0.1s, color 0.1s;
  }
  .ctx-item:hover { background: rgba(255,255,255,0.07); color: var(--color-text, #ccc); }
  .ctx-save { color: var(--color-accent, #007acc); }
  .ctx-save:hover { color: var(--color-accent, #007acc); }
  .ctx-danger:hover { background: rgba(196,43,28,0.15); color: #e06060; }
  .ctx-divider {
    height: 1px;
    background: color-mix(in srgb, var(--color-border, #404040) 50%, transparent);
    margin: 3px 0;
  }

  /* ── AI Server indicator ─────────────────────────────────────────────────── */
  .ai-server-btn {
    position: relative;
  }
  .ai-server-dot {
    position: absolute;
    top: 3px;
    right: 3px;
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: var(--color-textDim, #666);
    transition: background 0.2s;
  }
  .ai-server-btn.ai-running .ai-server-dot {
    background: #4caf50;
    animation: ai-pulse 1.5s ease-in-out infinite;
  }
  @keyframes ai-pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.4; }
  }
</style>
