/** Mirror of the Rust EditorCommand struct (returned by list_commands IPC). */
export interface EditorCommand {
  id: string;
  label: string;
  category: string;
  keybind?: string;
  description?: string;
}

/** UI-only command — never goes to Rust. */
export interface FrontendCommand {
  id: string;
  label: string;
  category: string;
  keybind?: string;
  description?: string;
  run: () => void | Promise<void>;
}

export type AnyCommand = EditorCommand | FrontendCommand;

export function isFrontendCommand(cmd: AnyCommand): cmd is FrontendCommand {
  return typeof (cmd as FrontendCommand).run === 'function';
}

export type OmnibarResult =
  | { kind: 'command'; command: AnyCommand }
  | { kind: 'entity'; id: number; name: string; components: string[] }
  | { kind: 'asset'; path: string; assetType: string }
  | { kind: 'recent'; label: string; path: string; itemType: 'project' | 'scene' };
