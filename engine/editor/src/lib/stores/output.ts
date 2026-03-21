// Output panel store — manages cargo build log lines and running state.
// Follows the module-level singleton pattern from console.ts.

export interface OutputLine {
  raw: string;                           // original with ANSI codes
  stream: 'stdout' | 'stderr';
  spans: Array<{ text: string; color: string | null; bold: boolean }>;
}

export interface OutputState {
  lines: OutputLine[];
  running: boolean;
  exitCode: number | null;
  cancelled: boolean;
  command: string | null;
}

let state: OutputState = {
  lines: [],
  running: false,
  exitCode: null,
  cancelled: false,
  command: null,
};
let listeners: (() => void)[] = [];

function notify() {
  listeners.forEach(fn => fn());
}

// 16-color palette (dark theme): indexed by color code offset from base
// Colors 30-37 use indices 0-7, colors 90-97 use indices 8-15
const PALETTE: string[] = [
  '#1e1e1e', // 30 black (dark)
  '#cc3e28', // 31 red
  '#57a64a', // 32 green
  '#d7ba7d', // 33 yellow
  '#569cd6', // 34 blue
  '#c586c0', // 35 magenta
  '#9cdcfe', // 36 cyan
  '#d4d4d4', // 37 white
  '#666666', // 90 bright black (gray)
  '#f44747', // 91 bright red
  '#b5cea8', // 92 bright green
  '#dcdcaa', // 93 bright yellow
  '#4ec9b0', // 94 bright blue/cyan
  '#d670d6', // 95 bright magenta
  '#87d5f5', // 96 bright cyan
  '#ffffff', // 97 bright white
];

interface Attrs { color: string | null; bold: boolean }

/** Parses ANSI SGR escape sequences. Handles colors 30-37, 90-97, bold (1), reset (0). */
function parseAnsi(raw: string): Array<{ text: string; color: string | null; bold: boolean }> {
  const spans: Array<{ text: string; color: string | null; bold: boolean }> = [];
  // Matches SGR sequences like \x1b[1;32m or \x1b[0m
  const re = /\x1b\[([0-9;]*)m/g;
  let cur: Attrs = { color: null, bold: false };
  let lastIndex = 0;
  let match: RegExpExecArray | null;

  while ((match = re.exec(raw)) !== null) {
    // Text before this escape sequence
    if (match.index > lastIndex) {
      const text = raw.slice(lastIndex, match.index);
      if (text) spans.push({ ...cur, text });
    }
    lastIndex = re.lastIndex;

    // Parse codes (may be "1;32" or "0" or "")
    const codes = match[1].split(';').map(Number);
    const next: Attrs = { ...cur };
    for (const code of codes) {
      if (code === 0 || match[1] === '') {
        next.color = null;
        next.bold = false;
      } else if (code === 1) {
        next.bold = true;
      } else if (code >= 30 && code <= 37) {
        next.color = PALETTE[code - 30];
      } else if (code >= 90 && code <= 97) {
        next.color = PALETTE[code - 90 + 8];
      }
    }
    cur = next;
  }

  // Remaining text after last escape
  if (lastIndex < raw.length) {
    const text = raw.slice(lastIndex);
    if (text) spans.push({ ...cur, text });
  }

  return spans.length > 0 ? spans : [{ text: raw, color: null, bold: false }];
}

export function getOutputState(): OutputState {
  return state;
}

export function subscribeOutput(fn: () => void): () => void {
  listeners.push(fn);
  return () => { listeners = listeners.filter(l => l !== fn); };
}

export function appendLine(raw: string, stream: 'stdout' | 'stderr'): void {
  const spans = parseAnsi(raw);
  state.lines = [...state.lines, { raw, stream, spans }];
  notify();
}

export function setRunning(cmd: string): void {
  state.running = true;
  state.command = cmd;
  state.exitCode = null;
  state.cancelled = false;
  notify();
}

export function setFinished(code: number | null, cancelled: boolean): void {
  state.running = false;
  state.exitCode = code;
  state.cancelled = cancelled;
  notify();
}

export function clearOutput(): void {
  state.lines = [];
  state.exitCode = null;
  state.cancelled = false;
  state.command = null;
  // Note: does NOT change `running` — safe to call mid-build
  notify();
}

/** For testing only. */
export function _resetForTest(): void {
  state = { lines: [], running: false, exitCode: null, cancelled: false, command: null };
  listeners = [];
}
