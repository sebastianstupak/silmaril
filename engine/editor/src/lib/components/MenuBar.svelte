<script lang="ts">
  import { t } from '$lib/i18n';
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu';

  interface Props {
    onSettingsOpen?: () => void;
    onOpenProject?: () => void;
    onLayoutReset?: () => void;
    onLayoutSelect?: (template: string) => void;
  }

  let { onSettingsOpen, onOpenProject, onLayoutReset, onLayoutSelect }: Props = $props();
</script>

<div class="menu-bar" role="menubar">
  <!-- File -->
  <DropdownMenu.Root>
    <DropdownMenu.Trigger class="menu-trigger">{t('menu.file')}</DropdownMenu.Trigger>
    <DropdownMenu.Content align="start" sideOffset={2} class="min-w-[200px]">
      <DropdownMenu.Item onclick={() => onOpenProject?.()}>{t('menu.file.open_project')}</DropdownMenu.Item>
      <DropdownMenu.Item>
        {t('menu.file.save_scene')}
        <DropdownMenu.Shortcut>Ctrl+S</DropdownMenu.Shortcut>
      </DropdownMenu.Item>
      <DropdownMenu.Item>{t('menu.file.save_scene_as')}</DropdownMenu.Item>
      <DropdownMenu.Item>{t('menu.file.new_scene')}</DropdownMenu.Item>
      <DropdownMenu.Sub>
        <DropdownMenu.SubTrigger>{t('menu.file.recent_projects')}</DropdownMenu.SubTrigger>
        <DropdownMenu.SubContent>
          <DropdownMenu.Item disabled>No recent projects</DropdownMenu.Item>
        </DropdownMenu.SubContent>
      </DropdownMenu.Sub>
      <DropdownMenu.Separator />
      <DropdownMenu.Item>{t('menu.file.exit')}</DropdownMenu.Item>
    </DropdownMenu.Content>
  </DropdownMenu.Root>

  <!-- Edit -->
  <DropdownMenu.Root>
    <DropdownMenu.Trigger class="menu-trigger">{t('menu.edit')}</DropdownMenu.Trigger>
    <DropdownMenu.Content align="start" sideOffset={2} class="min-w-[200px]">
      <DropdownMenu.Item>
        {t('menu.edit.undo')}
        <DropdownMenu.Shortcut>Ctrl+Z</DropdownMenu.Shortcut>
      </DropdownMenu.Item>
      <DropdownMenu.Item>
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
    </DropdownMenu.Content>
  </DropdownMenu.Root>

  <!-- View -->
  <DropdownMenu.Root>
    <DropdownMenu.Trigger class="menu-trigger">{t('menu.view')}</DropdownMenu.Trigger>
    <DropdownMenu.Content align="start" sideOffset={2} class="min-w-[200px]">
      <DropdownMenu.Sub>
        <DropdownMenu.SubTrigger>{t('menu.view.layout')}</DropdownMenu.SubTrigger>
        <DropdownMenu.SubContent>
          <DropdownMenu.Item onclick={() => onLayoutSelect?.('default')}>{t('layout.default')}</DropdownMenu.Item>
          <DropdownMenu.Item onclick={() => onLayoutSelect?.('tall')}>{t('layout.tall')}</DropdownMenu.Item>
          <DropdownMenu.Item onclick={() => onLayoutSelect?.('wide')}>{t('layout.wide')}</DropdownMenu.Item>
        </DropdownMenu.SubContent>
      </DropdownMenu.Sub>
      <DropdownMenu.Separator />
      <DropdownMenu.Item onclick={() => onLayoutReset?.()}>{t('layout.reset')}</DropdownMenu.Item>
    </DropdownMenu.Content>
  </DropdownMenu.Root>

  <!-- Entity -->
  <DropdownMenu.Root>
    <DropdownMenu.Trigger class="menu-trigger">{t('menu.entity')}</DropdownMenu.Trigger>
    <DropdownMenu.Content align="start" sideOffset={2} class="min-w-[200px]">
      <DropdownMenu.Item>{t('menu.entity.create_empty')}</DropdownMenu.Item>
      <DropdownMenu.Item>{t('menu.entity.create_from_template')}</DropdownMenu.Item>
      <DropdownMenu.Separator />
      <DropdownMenu.Item>{t('menu.entity.add_component')}</DropdownMenu.Item>
    </DropdownMenu.Content>
  </DropdownMenu.Root>

  <!-- Build -->
  <DropdownMenu.Root>
    <DropdownMenu.Trigger class="menu-trigger">{t('menu.build')}</DropdownMenu.Trigger>
    <DropdownMenu.Content align="start" sideOffset={2} class="min-w-[200px]">
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
    <DropdownMenu.Trigger class="menu-trigger">{t('menu.modules')}</DropdownMenu.Trigger>
    <DropdownMenu.Content align="start" sideOffset={2} class="min-w-[200px]">
      <DropdownMenu.Item>{t('menu.modules.add_module')}</DropdownMenu.Item>
      <DropdownMenu.Item>{t('menu.modules.manage_modules')}</DropdownMenu.Item>
    </DropdownMenu.Content>
  </DropdownMenu.Root>

  <!-- Help -->
  <DropdownMenu.Root>
    <DropdownMenu.Trigger class="menu-trigger">{t('menu.help')}</DropdownMenu.Trigger>
    <DropdownMenu.Content align="start" sideOffset={2} class="min-w-[200px]">
      <DropdownMenu.Item>{t('menu.help.documentation')}</DropdownMenu.Item>
      <DropdownMenu.Separator />
      <DropdownMenu.Item onclick={() => onSettingsOpen?.()}>{t('settings.title')}</DropdownMenu.Item>
      <DropdownMenu.Separator />
      <DropdownMenu.Item>{t('menu.help.about')}</DropdownMenu.Item>
    </DropdownMenu.Content>
  </DropdownMenu.Root>
</div>

<style>
  .menu-bar {
    display: flex;
    align-items: center;
    height: 2rem;
    background: var(--color-bgHeader, #2d2d2d);
    border-bottom: 1px solid var(--color-border, #404040);
    padding: 0 0.25rem;
    gap: 0;
    flex-shrink: 0;
  }
  .menu-bar :global(.menu-trigger) {
    all: unset;
    display: flex;
    align-items: center;
    padding: 0.125rem 0.5rem;
    font-size: 0.75rem;
    font-weight: 500;
    color: var(--color-text, #ccc);
    cursor: default;
    user-select: none;
    border-radius: 0.25rem;
    height: 1.5rem;
  }
  .menu-bar :global(.menu-trigger:hover),
  .menu-bar :global(.menu-trigger[data-state="open"]) {
    background: var(--color-bgPanel, #252525);
    color: var(--color-textBright, #fff);
  }
</style>
