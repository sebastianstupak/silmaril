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
  'menu.file.no_recent_projects': 'No recent projects',
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
  'menu.view.layout': 'Layout',
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
  'panel.file_explorer': 'File Explorer',

  // Placeholders
  'placeholder.no_project': 'No project loaded',
  'placeholder.select_entity': 'Select an entity',
  'placeholder.no_logs': 'No logs yet',
  'placeholder.viewport': 'Vulkan viewport will render here',
  'viewport.loading': 'Initializing viewport...',
  'viewport.no_entities': 'No entities in scene',
  'viewport.zoom': 'Zoom',
  'viewport.reset_camera': 'Reset Camera',

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
  'settings.compact_menu': 'Compact menu',
  'settings.compact_menu.description': 'Show icons only in the title bar menu',

  // Keybindings tab
  'keybindings.layout_slots': 'Layout Slots',
  'keybindings.press_key': 'Press a key…',
  'keybindings.conflict': 'Already used by "{name}"',
  'keybindings.clear': 'Clear',
  'keybindings.none': 'None',

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

  // Console
  'console.clear': 'Clear',
  'console.filter': 'Filter logs...',
  'console.info': 'Info',
  'console.warn': 'Warn',
  'console.error': 'Error',
  'console.debug': 'Debug',
  'console.no_logs': 'No logs',

  // File explorer
  'explorer.new_file': 'New File',
  'explorer.new_folder': 'New Folder',
  'explorer.rename': 'Rename',
  'explorer.delete': 'Delete',
  'explorer.copy_path': 'Copy Path',
  'explorer.reveal': 'Reveal in Explorer',
  'explorer.refresh': 'Refresh',
  'explorer.show_ignored': 'Show Ignored Files',
  'explorer.empty': 'No files',
  'explorer.loading': 'Loading...',
  'explorer.error': 'Could not read folder',

  // Hierarchy panel
  'hierarchy.search': 'Search entities...',
  'hierarchy.empty': 'No entities',
  'hierarchy.count': '{count} entities',

  // Inspector panel
  'inspector.no_selection': 'No entity selected',
  'inspector.components': 'Components',
  'inspector.add_component': 'Add Component',

  // Dialog
  'dialog.open_project_title': 'Open Silmaril Project',

  // Layout
  'layout.default': 'Default',
  'layout.tall': 'Tall',
  'layout.wide': 'Wide',
  'layout.reset': 'Reset Layout',
  'layout.save': 'Save Layout',

  // Docking
  'dock.close_tab': 'Close',
  'dock.drop_here': 'Drop here',
  'dock.pop_out': 'Pop Out',
  'dock.duplicate': 'Duplicate Panel',
  'dock.close_others': 'Close Others',
  'dock.close_all': 'Close All',

  // Scene tools
  'tool.select': 'Select (Q)',
  'tool.move': 'Move (W)',
  'tool.rotate': 'Rotate (E)',
  'tool.scale': 'Scale (R)',
  'tool.focus': 'Focus (F)',

  // Viewport scene controls
  'viewport.grid': 'Toggle Grid',
  'viewport.snap': 'Snap to Grid',
  'viewport.orbit': 'Orbit',
  'viewport.pan': 'Pan',

  // Scene entity operations
  'scene.create_entity': 'Create Entity',
  'scene.delete_entity': 'Delete Entity',
  'scene.duplicate_entity': 'Duplicate Entity',

  // Pop-out
  'popout.unknown': 'Unknown panel',
  'popout.dock_back': 'Dock Back',

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
