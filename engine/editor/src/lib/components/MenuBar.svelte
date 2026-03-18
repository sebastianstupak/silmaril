<script lang="ts">
  import { t } from '$lib/i18n';
  import * as Menubar from '$lib/components/ui/menubar';

  interface Props {
    onSettingsOpen?: () => void;
    onOpenProject?: () => void;
  }

  let { onSettingsOpen, onOpenProject }: Props = $props();

  // View panel toggle state
  let showHierarchy = $state(true);
  let showInspector = $state(true);
  let showConsole = $state(true);
  let showViewport = $state(true);
</script>

<Menubar.Root class="h-8 rounded-none border-x-0 border-t-0 bg-[var(--color-bgHeader,#2d2d2d)] shadow-none">
  <!-- File -->
  <Menubar.Menu>
    <Menubar.Trigger class="text-xs">{t('menu.file')}</Menubar.Trigger>
    <Menubar.Content>
      <Menubar.Item onclick={() => onOpenProject?.()}>{t('menu.file.open_project')}</Menubar.Item>
      <Menubar.Item>
        {t('menu.file.save_scene')}
        <Menubar.Shortcut>Ctrl+S</Menubar.Shortcut>
      </Menubar.Item>
      <Menubar.Item>{t('menu.file.save_scene_as')}</Menubar.Item>
      <Menubar.Item>{t('menu.file.new_scene')}</Menubar.Item>
      <Menubar.Sub>
        <Menubar.SubTrigger>{t('menu.file.recent_projects')}</Menubar.SubTrigger>
        <Menubar.SubContent>
          <Menubar.Item disabled>No recent projects</Menubar.Item>
        </Menubar.SubContent>
      </Menubar.Sub>
      <Menubar.Separator />
      <Menubar.Item>{t('menu.file.exit')}</Menubar.Item>
    </Menubar.Content>
  </Menubar.Menu>

  <!-- Edit -->
  <Menubar.Menu>
    <Menubar.Trigger class="text-xs">{t('menu.edit')}</Menubar.Trigger>
    <Menubar.Content>
      <Menubar.Item>
        {t('menu.edit.undo')}
        <Menubar.Shortcut>Ctrl+Z</Menubar.Shortcut>
      </Menubar.Item>
      <Menubar.Item>
        {t('menu.edit.redo')}
        <Menubar.Shortcut>Ctrl+Y</Menubar.Shortcut>
      </Menubar.Item>
      <Menubar.Separator />
      <Menubar.Item>
        {t('menu.edit.cut')}
        <Menubar.Shortcut>Ctrl+X</Menubar.Shortcut>
      </Menubar.Item>
      <Menubar.Item>
        {t('menu.edit.copy')}
        <Menubar.Shortcut>Ctrl+C</Menubar.Shortcut>
      </Menubar.Item>
      <Menubar.Item>
        {t('menu.edit.paste')}
        <Menubar.Shortcut>Ctrl+V</Menubar.Shortcut>
      </Menubar.Item>
      <Menubar.Separator />
      <Menubar.Item>
        {t('menu.edit.duplicate')}
        <Menubar.Shortcut>Ctrl+D</Menubar.Shortcut>
      </Menubar.Item>
      <Menubar.Item>
        {t('menu.edit.delete')}
        <Menubar.Shortcut>Del</Menubar.Shortcut>
      </Menubar.Item>
      <Menubar.Item>
        {t('menu.edit.select_all')}
        <Menubar.Shortcut>Ctrl+A</Menubar.Shortcut>
      </Menubar.Item>
    </Menubar.Content>
  </Menubar.Menu>

  <!-- View -->
  <Menubar.Menu>
    <Menubar.Trigger class="text-xs">{t('menu.view')}</Menubar.Trigger>
    <Menubar.Content>
      <Menubar.CheckboxItem bind:checked={showHierarchy}>
        {t('menu.view.hierarchy')}
      </Menubar.CheckboxItem>
      <Menubar.CheckboxItem bind:checked={showInspector}>
        {t('menu.view.inspector')}
      </Menubar.CheckboxItem>
      <Menubar.CheckboxItem bind:checked={showConsole}>
        {t('menu.view.console')}
      </Menubar.CheckboxItem>
      <Menubar.CheckboxItem bind:checked={showViewport}>
        {t('menu.view.viewport')}
      </Menubar.CheckboxItem>
      <Menubar.Separator />
      <Menubar.Item>{t('menu.view.reset_layout')}</Menubar.Item>
    </Menubar.Content>
  </Menubar.Menu>

  <!-- Entity -->
  <Menubar.Menu>
    <Menubar.Trigger class="text-xs">{t('menu.entity')}</Menubar.Trigger>
    <Menubar.Content>
      <Menubar.Item>{t('menu.entity.create_empty')}</Menubar.Item>
      <Menubar.Item>{t('menu.entity.create_from_template')}</Menubar.Item>
      <Menubar.Separator />
      <Menubar.Item>{t('menu.entity.add_component')}</Menubar.Item>
    </Menubar.Content>
  </Menubar.Menu>

  <!-- Build -->
  <Menubar.Menu>
    <Menubar.Trigger class="text-xs">{t('menu.build')}</Menubar.Trigger>
    <Menubar.Content>
      <Menubar.Item>
        {t('menu.build.build_project')}
        <Menubar.Shortcut>Ctrl+B</Menubar.Shortcut>
      </Menubar.Item>
      <Menubar.Item>{t('menu.build.build_release')}</Menubar.Item>
      <Menubar.Separator />
      <Menubar.Item>{t('menu.build.package')}</Menubar.Item>
      <Menubar.Item>{t('menu.build.platform_settings')}</Menubar.Item>
    </Menubar.Content>
  </Menubar.Menu>

  <!-- Modules -->
  <Menubar.Menu>
    <Menubar.Trigger class="text-xs">{t('menu.modules')}</Menubar.Trigger>
    <Menubar.Content>
      <Menubar.Item>{t('menu.modules.add_module')}</Menubar.Item>
      <Menubar.Item>{t('menu.modules.manage_modules')}</Menubar.Item>
    </Menubar.Content>
  </Menubar.Menu>

  <!-- Help -->
  <Menubar.Menu>
    <Menubar.Trigger class="text-xs">{t('menu.help')}</Menubar.Trigger>
    <Menubar.Content>
      <Menubar.Item>{t('menu.help.documentation')}</Menubar.Item>
      <Menubar.Separator />
      <Menubar.Item onclick={() => onSettingsOpen?.()}>{t('settings.title')}</Menubar.Item>
      <Menubar.Separator />
      <Menubar.Item>{t('menu.help.about')}</Menubar.Item>
    </Menubar.Content>
  </Menubar.Menu>
</Menubar.Root>
