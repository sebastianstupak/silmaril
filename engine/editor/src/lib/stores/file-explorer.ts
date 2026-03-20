// File explorer store — manages the project file tree state.

import { invoke } from '@tauri-apps/api/core';

export interface TreeNode {
  name: string;
  path: string;
  kind: 'file' | 'dir';
  children: TreeNode[] | null; // null = not expanded yet
  git_status: string | null;
  ignored: boolean;
}

export interface FileExplorerState {
  root: string | null;
  nodes: TreeNode[];
  expanded: Set<string>;
  selected: string | null;
  gitStatus: Record<string, string>;
  showIgnored: boolean;
  loading: boolean;
  error: string | null;
}

let state: FileExplorerState = {
  root: null,
  nodes: [],
  expanded: new Set(),
  selected: null,
  gitStatus: {},
  showIgnored: false,
  loading: false,
  error: null,
};

let listeners: (() => void)[] = [];

function notify() {
  listeners.forEach((fn) => fn());
}

export function getFileExplorerState(): FileExplorerState {
  return state;
}

export function subscribeFileExplorer(fn: () => void): () => void {
  listeners.push(fn);
  return () => {
    listeners = listeners.filter((l) => l !== fn);
  };
}

export async function loadTree(root: string): Promise<void> {
  state = { ...state, root, loading: true, error: null };
  notify();
  try {
    const nodes = await invoke<TreeNode[]>('get_file_tree', { root });
    const gitStatus = await invoke<Record<string, string>>('get_git_status', { root });
    state = { ...state, nodes, gitStatus, loading: false };
  } catch (e) {
    state = { ...state, loading: false, error: String(e) };
  }
  notify();
}

export async function expandDir(path: string): Promise<void> {
  try {
    const children = await invoke<TreeNode[]>('expand_dir', { path });
    state = {
      ...state,
      expanded: new Set([...state.expanded, path]),
      nodes: patchChildren(state.nodes, path, children),
    };
  } catch (e) {
    state = { ...state, error: String(e) };
  }
  notify();
}

export function collapseDir(path: string): void {
  const next = new Set(state.expanded);
  next.delete(path);
  state = { ...state, expanded: next };
  notify();
}

export function setSelected(path: string | null): void {
  state = { ...state, selected: path };
  notify();
}

export function toggleShowIgnored(): void {
  state = { ...state, showIgnored: !state.showIgnored };
  notify();
}

export async function refreshTree(): Promise<void> {
  if (!state.root) return;
  await loadTree(state.root);
}

/** Recursively replace children of the node at targetPath */
function patchChildren(
  nodes: TreeNode[],
  targetPath: string,
  children: TreeNode[],
): TreeNode[] {
  return nodes.map((node) => {
    if (node.path === targetPath) return { ...node, children };
    if (node.children)
      return { ...node, children: patchChildren(node.children, targetPath, children) };
    return node;
  });
}
