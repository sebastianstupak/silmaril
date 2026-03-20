<script lang="ts">
  import { t } from '$lib/i18n';
  import { getAvailableLocales, setLocale } from '$lib/i18n';
  import { themes, applyTheme } from '$lib/theme/tokens';
  import { saveSettings, type EditorSettings } from '$lib/stores/settings';
  import { captureKeybind, formatKeybindDisplay, findKeybindConflict } from '$lib/keybind-utils';
  import type { SavedLayout } from '$lib/docking/store';
  import * as Dialog from '$lib/components/ui/dialog';
  import * as Tabs from '$lib/components/ui/tabs';
  import * as Select from '$lib/components/ui/select';
  import { Label } from '$lib/components/ui/label';

  interface Props {
    open: boolean;
    settings: EditorSettings;
    savedLayouts?: SavedLayout[];
    onOpenChange?: (open: boolean) => void;
    onSettingsChange?: (settings: EditorSettings) => void;
    onUpdateLayoutKeybind?: (id: string, keybind: string | undefined) => void;
  }

  let {
    open = $bindable(false),
    settings,
    savedLayouts = [],
    onOpenChange,
    onSettingsChange,
    onUpdateLayoutKeybind,
  }: Props = $props();

  let activeTab = $state('general');

  // ── Keybinding capture state ───────────────────────────────────────────────
  /** Auto-focus a node as soon as it mounts in the DOM. */
  function autofocus(node: HTMLElement) {
    node.focus();
  }

  /** id of the slot currently being rebound, null if none */
  let capturingId: string | null = $state(null);
  /** pending conflict slot during capture */
  let captureConflict: SavedLayout | undefined = $state(undefined);

  function startCapture(id: string) {
    capturingId = id;
    captureConflict = undefined;
  }

  function cancelCapture() {
    capturingId = null;
    captureConflict = undefined;
  }

  function handleKeybindKeydown(e: KeyboardEvent, id: string) {
    e.preventDefault();
    e.stopPropagation();

    if (e.key === 'Escape') { cancelCapture(); return; }

    const keybind = captureKeybind(e);
    if (!keybind) return; // modifier-only, keep waiting

    const conflict = findKeybindConflict(keybind, id, savedLayouts);
    if (conflict) {
      captureConflict = conflict;
      return; // show conflict, keep capturing until Escape or valid key
    }

    onUpdateLayoutKeybind?.(id, keybind);
    cancelCapture();
  }

  function clearKeybind(id: string) {
    onUpdateLayoutKeybind?.(id, undefined);
  }

  // ── Settings helpers ───────────────────────────────────────────────────────
  function updateSetting<K extends keyof EditorSettings>(key: K, value: EditorSettings[K]) {
    const updated = { ...settings, [key]: value };
    onSettingsChange?.(updated);
    saveSettings(updated);

    if (key === 'theme') {
      applyTheme(themes[value as string] ?? themes.dark);
    }
    if (key === 'language') {
      setLocale(value as string);
    }
    if (key === 'fontSize') {
      document.documentElement.style.fontSize = `${value}px`;
    }
  }

  function handleOpenChange(value: boolean) {
    open = value;
    if (!value) cancelCapture();
    onOpenChange?.(value);
  }
</script>

<Dialog.Root bind:open onOpenChange={handleOpenChange}>
  <Dialog.Content class="sm:max-w-[600px] max-h-[80vh]">
    <Dialog.Header>
      <Dialog.Title>{t('settings.title')}</Dialog.Title>
      <Dialog.Description class="sr-only">{t('settings.title')}</Dialog.Description>
    </Dialog.Header>

    <Tabs.Root bind:value={activeTab} class="flex flex-row gap-4 min-h-[300px]" orientation="vertical">
      <Tabs.List class="flex flex-col h-auto w-40 shrink-0 rounded-md bg-[var(--color-bgPanel,#252525)]">
        <Tabs.Trigger value="general"     class="justify-start w-full">{t('settings.general')}</Tabs.Trigger>
        <Tabs.Trigger value="appearance"  class="justify-start w-full">{t('settings.appearance')}</Tabs.Trigger>
        <Tabs.Trigger value="editor"      class="justify-start w-full">{t('settings.editor')}</Tabs.Trigger>
        <Tabs.Trigger value="keybindings" class="justify-start w-full">{t('settings.keybindings')}</Tabs.Trigger>
      </Tabs.List>

      <!-- General -->
      <Tabs.Content value="general" class="flex-1 space-y-4 pt-1">
        <div class="grid gap-2">
          <Label>{t('settings.language')}</Label>
          <Select.Root
            type="single"
            value={settings.language}
            onValueChange={(v) => { if (v) updateSetting('language', v); }}
          >
            <Select.Trigger class="w-full">
              {settings.language.toUpperCase()}
            </Select.Trigger>
            <Select.Content>
              {#each getAvailableLocales() as locale}
                <Select.Item value={locale} label={locale.toUpperCase()}>
                  {locale.toUpperCase()}
                </Select.Item>
              {/each}
            </Select.Content>
          </Select.Root>
        </div>

        <div class="grid gap-2">
          <Label>{t('settings.auto_save')}</Label>
          <Select.Root
            type="single"
            value={settings.autoSave}
            onValueChange={(v) => { if (v) updateSetting('autoSave', v as EditorSettings['autoSave']); }}
          >
            <Select.Trigger class="w-full">
              {t(`settings.auto_save.${settings.autoSave}`)}
            </Select.Trigger>
            <Select.Content>
              <Select.Item value="off"             label={t('settings.auto_save.off')}>
                {t('settings.auto_save.off')}
              </Select.Item>
              <Select.Item value="on_focus_change" label={t('settings.auto_save.on_focus_change')}>
                {t('settings.auto_save.on_focus_change')}
              </Select.Item>
              <Select.Item value="after_delay"     label={t('settings.auto_save.after_delay')}>
                {t('settings.auto_save.after_delay')}
              </Select.Item>
            </Select.Content>
          </Select.Root>
        </div>
      </Tabs.Content>

      <!-- Appearance -->
      <Tabs.Content value="appearance" class="flex-1 space-y-4 pt-1">
        <div class="grid gap-2">
          <Label>{t('settings.theme')}</Label>
          <Select.Root
            type="single"
            value={settings.theme}
            onValueChange={(v) => { if (v) updateSetting('theme', v); }}
          >
            <Select.Trigger class="w-full">
              {t(`settings.theme.${settings.theme}`)}
            </Select.Trigger>
            <Select.Content>
              <Select.Item value="dark"  label={t('settings.theme.dark')}>
                {t('settings.theme.dark')}
              </Select.Item>
              <Select.Item value="light" label={t('settings.theme.light')}>
                {t('settings.theme.light')}
              </Select.Item>
            </Select.Content>
          </Select.Root>
        </div>

        <div class="grid gap-2">
          <Label>{t('settings.font_size')}</Label>
          <div class="flex items-center gap-3">
            <input
              type="range"
              min="10"
              max="18"
              step="1"
              value={settings.fontSize}
              oninput={(e) => updateSetting('fontSize', parseInt((e.target as HTMLInputElement).value))}
              class="flex-1 h-2 rounded-full appearance-none bg-[var(--color-bgInput,#333)] cursor-pointer
                [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:h-4 [&::-webkit-slider-thumb]:w-4
                [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-[var(--color-accent,#007acc)]"
            />
            <span class="text-sm text-muted-foreground w-10 text-right">{settings.fontSize}px</span>
          </div>
        </div>
      </Tabs.Content>

      <!-- Editor -->
      <Tabs.Content value="editor" class="flex-1 pt-1 space-y-4">
        <div class="flex items-center justify-between gap-4">
          <div class="flex flex-col gap-0.5">
            <Label>Compact menu</Label>
            <p class="text-xs text-muted-foreground">Show icons only in the title bar menu</p>
          </div>
          <button
            role="switch"
            aria-checked={settings.compactMenu}
            class="w-9 h-5 rounded-full border border-[var(--color-border,#404040)] relative transition-colors
              {settings.compactMenu
                ? 'bg-[var(--color-accent,#007acc)]'
                : 'bg-[var(--color-bgInput,#333)]'}"
            onclick={() => updateSetting('compactMenu', !settings.compactMenu)}
          >
            <span class="block w-3.5 h-3.5 rounded-full bg-white absolute top-0.5 transition-transform
              {settings.compactMenu ? 'translate-x-4' : 'translate-x-0.5'}">
            </span>
          </button>
        </div>
      </Tabs.Content>

      <!-- Keybindings -->
      <Tabs.Content value="keybindings" class="flex-1 pt-1 overflow-y-auto">
        <!-- Layout Slots section -->
        <div class="space-y-3">
          <p class="text-xs font-semibold uppercase tracking-wider text-[var(--color-textMuted,#999)]">
            {t('keybindings.layout_slots')}
          </p>

          {#each savedLayouts as slot (slot.id)}
            {@const isCapturing = capturingId === slot.id}
            <div class="keybind-row">
              <!-- Slot name -->
              <span class="keybind-label">{slot.name}</span>

              <!-- Keybind button / capture input -->
              <div class="keybind-control">
                {#if isCapturing}
                  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
                  <div
                    class="keybind-capture"
                    tabindex="0"
                    role="button"
                    aria-label={t('keybindings.press_key')}
                    onkeydown={(e) => handleKeybindKeydown(e, slot.id)}
                    onblur={cancelCapture}
                    use:autofocus
                  >
                    {#if captureConflict}
                      <span class="keybind-conflict">
                        {t('keybindings.conflict').replace('{name}', captureConflict.name)}
                      </span>
                    {:else}
                      <span class="keybind-prompt">{t('keybindings.press_key')}</span>
                    {/if}
                  </div>
                {:else}
                  <button
                    class="keybind-badge"
                    onclick={() => startCapture(slot.id)}
                    title="Click to change"
                  >
                    {slot.keybind ? formatKeybindDisplay(slot.keybind) : t('keybindings.none')}
                  </button>
                {/if}

                {#if slot.keybind && !isCapturing}
                  <button
                    class="keybind-clear"
                    onclick={() => clearKeybind(slot.id)}
                    title={t('keybindings.clear')}
                    aria-label={t('keybindings.clear')}
                  >×</button>
                {/if}
              </div>
            </div>
          {/each}
        </div>
      </Tabs.Content>
    </Tabs.Root>
  </Dialog.Content>
</Dialog.Root>

<style>
  .keybind-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px 2px;
    border-bottom: 1px solid var(--color-border, #333);
    gap: 12px;
  }
  .keybind-row:last-child {
    border-bottom: none;
  }

  .keybind-label {
    font-size: 13px;
    color: var(--color-text, #ccc);
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .keybind-control {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
  }

  .keybind-badge {
    font-size: 12px;
    font-family: var(--font-mono, monospace);
    padding: 2px 8px;
    border: 1px solid var(--color-border, #444);
    border-radius: 4px;
    background: var(--color-bgInput, #2a2a2a);
    color: var(--color-text, #ccc);
    cursor: pointer;
    min-width: 60px;
    text-align: center;
  }
  .keybind-badge:hover {
    border-color: var(--color-accent, #007acc);
    color: var(--color-accent, #007acc);
  }

  .keybind-capture {
    font-size: 12px;
    font-family: var(--font-mono, monospace);
    padding: 2px 8px;
    border: 1px solid var(--color-accent, #007acc);
    border-radius: 4px;
    background: var(--color-bgInput, #2a2a2a);
    color: var(--color-accent, #007acc);
    min-width: 120px;
    text-align: center;
    outline: none;
    cursor: default;
    animation: pulse-border 1.5s ease-in-out infinite;
  }

  .keybind-prompt {
    opacity: 0.7;
  }

  .keybind-conflict {
    color: var(--color-warn, #e5a000);
    font-size: 11px;
  }

  .keybind-clear {
    width: 18px;
    height: 18px;
    border-radius: 50%;
    border: none;
    background: var(--color-bgHover, #333);
    color: var(--color-textMuted, #888);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 14px;
    line-height: 1;
    padding: 0;
  }
  .keybind-clear:hover {
    background: var(--color-danger, #c62828);
    color: #fff;
  }

  @keyframes pulse-border {
    0%, 100% { box-shadow: 0 0 0 0 color-mix(in srgb, var(--color-accent, #007acc) 40%, transparent); }
    50%       { box-shadow: 0 0 0 3px color-mix(in srgb, var(--color-accent, #007acc) 0%, transparent); }
  }
</style>
