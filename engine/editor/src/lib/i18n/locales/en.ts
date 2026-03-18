// English translations — this is the source of truth for all translation keys

const en = {
  // App
  'app.title': 'Silmaril Editor',

  // Menu bar
  'menu.file': 'File',
  'menu.file.open_project': 'Open Project...',
  'menu.file.save_scene': 'Save Scene',
  'menu.file.save_scene_as': 'Save Scene As...',
  'menu.file.new_scene': 'New Scene',
  'menu.file.recent_projects': 'Recent Projects',
  'menu.file.exit': 'Exit',

  'menu.edit': 'Edit',
  'menu.edit.undo': 'Undo',
  'menu.edit.redo': 'Redo',
  'menu.edit.cut': 'Cut',
  'menu.edit.copy': 'Copy',
  'menu.edit.paste': 'Paste',
  'menu.edit.duplicate': 'Duplicate',
  'menu.edit.delete': 'Delete',
  'menu.edit.select_all': 'Select All',

  'menu.view': 'View',
  'menu.view.hierarchy': 'Hierarchy',
  'menu.view.inspector': 'Inspector',
  'menu.view.console': 'Console',
  'menu.view.viewport': 'Viewport',
  'menu.view.profiler': 'Profiler',
  'menu.view.reset_layout': 'Reset Layout',

  'menu.entity': 'Entity',
  'menu.entity.create_empty': 'Create Empty',
  'menu.entity.create_from_template': 'Create from Template...',
  'menu.entity.add_component': 'Add Component...',

  'menu.build': 'Build',
  'menu.build.build_project': 'Build Project',
  'menu.build.build_release': 'Build Release',
  'menu.build.package': 'Package...',
  'menu.build.platform_settings': 'Platform Settings...',

  'menu.modules': 'Modules',
  'menu.modules.add_module': 'Add Module...',
  'menu.modules.manage_modules': 'Manage Modules...',

  'menu.help': 'Help',
  'menu.help.documentation': 'Documentation',
  'menu.help.about': 'About Silmaril Editor',

  // Toolbar
  'toolbar.play': 'Play',
  'toolbar.pause': 'Pause',
  'toolbar.stop': 'Stop',

  // Panels
  'panel.hierarchy': 'Hierarchy',
  'panel.inspector': 'Inspector',
  'panel.viewport': 'Viewport',
  'panel.console': 'Console',
  'panel.profiler': 'Profiler',
  'panel.assets': 'Assets',

  // Placeholders
  'placeholder.no_project': 'No project loaded',
  'placeholder.select_entity': 'Select an entity',
  'placeholder.no_logs': 'No logs yet',
  'placeholder.viewport': 'Vulkan viewport will render here',

  // Settings dialog
  'settings.title': 'Settings',
  'settings.general': 'General',
  'settings.appearance': 'Appearance',
  'settings.editor': 'Editor',
  'settings.keybindings': 'Keybindings',

  'settings.language': 'Language',
  'settings.theme': 'Theme',
  'settings.theme.dark': 'Dark',
  'settings.theme.light': 'Light',
  'settings.font_size': 'Font Size',
  'settings.auto_save': 'Auto Save',
  'settings.auto_save.off': 'Off',
  'settings.auto_save.on_focus_change': 'On Focus Change',
  'settings.auto_save.after_delay': 'After Delay',

  // Modes
  'mode.edit': 'Edit',
  'mode.play': 'Play',
  'mode.pause': 'Pause',

  // Shortcuts (displayed in menus)
  'shortcut.ctrl': 'Ctrl',
  'shortcut.shift': 'Shift',
  'shortcut.alt': 'Alt',

  // Status bar
  'status.ready': 'Ready',
  'status.building': 'Building...',
  'status.saved': 'Scene saved',
  'status.fps': 'FPS',
  'status.memory': 'Memory',

  // Panel menu
  'panel.menu': 'Panel options',

  // Breadcrumb
  'breadcrumb.no_project': 'No project',
  'breadcrumb.no_scene': 'Untitled Scene',

  // Common
  'common.ok': 'OK',
  'common.cancel': 'Cancel',
  'common.apply': 'Apply',
  'common.close': 'Close',
  'common.save': 'Save',
  'common.open': 'Open',
  'common.delete': 'Delete',
} as const;

export default en;
export type TranslationKeys = typeof en;
