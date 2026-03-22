// engine/editor/src/lib/contributions/builtins.ts
import { registerPanel } from './registry';
import HierarchyWrapper from '../docking/panels/HierarchyWrapper.svelte';
import ViewportPanel from '../docking/panels/ViewportPanel.svelte';
import InspectorWrapper from '../docking/panels/InspectorWrapper.svelte';
import ConsoleWrapper from '../docking/panels/ConsoleWrapper.svelte';
import ProfilerPanel from '../docking/panels/ProfilerPanel.svelte';
import AssetsPanel from '../docking/panels/AssetsPanel.svelte';
import FileExplorerWrapper from '../docking/panels/FileExplorerWrapper.svelte';
import TerminalWrapper from '../docking/panels/TerminalWrapper.svelte';
import OutputWrapper from '../docking/panels/OutputWrapper.svelte';

export function registerBuiltinPanels(): void {
  registerPanel({ id: 'hierarchy',     title: 'Hierarchy',     component: HierarchyWrapper as any,    source: 'builtin' });
  registerPanel({ id: 'viewport',      title: 'Viewport',      component: ViewportPanel as any,        source: 'builtin' });
  registerPanel({ id: 'inspector',     title: 'Inspector',     component: InspectorWrapper as any,     source: 'builtin' });
  registerPanel({ id: 'console',       title: 'Console',       component: ConsoleWrapper as any,       source: 'builtin' });
  registerPanel({ id: 'profiler',      title: 'Profiler',      component: ProfilerPanel as any,        source: 'builtin' });
  registerPanel({ id: 'assets',        title: 'Assets',        component: AssetsPanel as any,          source: 'builtin' });
  registerPanel({ id: 'file-explorer', title: 'File Explorer', component: FileExplorerWrapper as any,  source: 'builtin' });
  registerPanel({ id: 'terminal',      title: 'Terminal',      component: TerminalWrapper as any,      source: 'builtin' });
  registerPanel({ id: 'output',        title: 'Output',        component: OutputWrapper as any,        source: 'builtin' });
}
