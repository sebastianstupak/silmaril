<script lang="ts">
  import { t } from '$lib/i18n';
  import { getAvailableLocales, setLocale } from '$lib/i18n';
  import { themes, applyTheme } from '$lib/theme/tokens';
  import { saveSettings, type EditorSettings } from '$lib/stores/settings';
  import * as Dialog from '$lib/components/ui/dialog';
  import * as Tabs from '$lib/components/ui/tabs';
  import * as Select from '$lib/components/ui/select';
  import { Label } from '$lib/components/ui/label';

  interface Props {
    open: boolean;
    settings: EditorSettings;
    onOpenChange?: (open: boolean) => void;
    onSettingsChange?: (settings: EditorSettings) => void;
  }

  let { open = $bindable(false), settings, onOpenChange, onSettingsChange }: Props = $props();

  let activeTab = $state('general');

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
        <Tabs.Trigger value="general" class="justify-start w-full">{t('settings.general')}</Tabs.Trigger>
        <Tabs.Trigger value="appearance" class="justify-start w-full">{t('settings.appearance')}</Tabs.Trigger>
        <Tabs.Trigger value="editor" class="justify-start w-full">{t('settings.editor')}</Tabs.Trigger>
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
              <Select.Item value="off" label={t('settings.auto_save.off')}>
                {t('settings.auto_save.off')}
              </Select.Item>
              <Select.Item value="on_focus_change" label={t('settings.auto_save.on_focus_change')}>
                {t('settings.auto_save.on_focus_change')}
              </Select.Item>
              <Select.Item value="after_delay" label={t('settings.auto_save.after_delay')}>
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
              <Select.Item value="dark" label={t('settings.theme.dark')}>
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

      <!-- Editor (placeholder) -->
      <Tabs.Content value="editor" class="flex-1 pt-1">
        <p class="text-sm text-muted-foreground italic">{t('settings.editor')}</p>
      </Tabs.Content>

      <!-- Keybindings (placeholder) -->
      <Tabs.Content value="keybindings" class="flex-1 pt-1">
        <p class="text-sm text-muted-foreground italic">{t('settings.keybindings')}</p>
      </Tabs.Content>
    </Tabs.Root>
  </Dialog.Content>
</Dialog.Root>
