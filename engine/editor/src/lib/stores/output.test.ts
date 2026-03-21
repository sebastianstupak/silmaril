import { describe, it, expect, beforeEach } from 'vitest';
import {
  getOutputState,
  appendLine,
  setRunning,
  setFinished,
  clearOutput,
  _resetForTest,
} from './output';

beforeEach(() => _resetForTest());

describe('ANSI parser — standard colors', () => {
  it('parses red foreground', () => {
    appendLine('\x1b[31mhello\x1b[0m', 'stdout');
    const spans = getOutputState().lines[0].spans;
    expect(spans[0].text).toBe('hello');
    expect(spans[0].color).not.toBeNull();
    expect(spans[0].bold).toBe(false);
  });

  it('parses all standard foreground colors (30-37)', () => {
    for (let i = 30; i <= 37; i++) {
      _resetForTest();
      appendLine(`\x1b[${i}mtext\x1b[0m`, 'stdout');
      const span = getOutputState().lines[0].spans[0];
      expect(span.text).toBe('text');
      expect(span.color).not.toBeNull();
    }
  });

  it('parses bright foreground colors (90-97)', () => {
    for (let i = 90; i <= 97; i++) {
      _resetForTest();
      appendLine(`\x1b[${i}mtext\x1b[0m`, 'stdout');
      const span = getOutputState().lines[0].spans[0];
      expect(span.text).toBe('text');
      expect(span.color).not.toBeNull();
    }
  });

  it('parses bold', () => {
    appendLine('\x1b[1mhello\x1b[0m', 'stdout');
    expect(getOutputState().lines[0].spans[0].bold).toBe(true);
  });

  it('resets on code 0', () => {
    appendLine('\x1b[31mred\x1b[0mnormal', 'stdout');
    const spans = getOutputState().lines[0].spans;
    expect(spans[0].color).not.toBeNull();
    expect(spans[1].color).toBeNull();
    expect(spans[1].bold).toBe(false);
  });

  it('handles mixed bold + color', () => {
    appendLine('\x1b[1;32mbold green\x1b[0m', 'stdout');
    const span = getOutputState().lines[0].spans[0];
    expect(span.bold).toBe(true);
    expect(span.color).not.toBeNull();
  });

  it('plain text has null color and false bold', () => {
    appendLine('plain text', 'stdout');
    const span = getOutputState().lines[0].spans[0];
    expect(span.text).toBe('plain text');
    expect(span.color).toBeNull();
    expect(span.bold).toBe(false);
  });
});

describe('appendLine', () => {
  it('stores raw string and stream discriminator', () => {
    appendLine('\x1b[31merror\x1b[0m', 'stderr');
    const line = getOutputState().lines[0];
    expect(line.raw).toBe('\x1b[31merror\x1b[0m');
    expect(line.stream).toBe('stderr');
  });
});

describe('state transitions', () => {
  it('setRunning sets running=true and command', () => {
    setRunning('cargo build');
    const s = getOutputState();
    expect(s.running).toBe(true);
    expect(s.command).toBe('cargo build');
  });

  it('setFinished sets running=false and exitCode', () => {
    setRunning('cargo build');
    setFinished(0, false);
    const s = getOutputState();
    expect(s.running).toBe(false);
    expect(s.exitCode).toBe(0);
    expect(s.cancelled).toBe(false);
  });

  it('clearOutput resets lines, exitCode, cancelled, command but does not touch running', () => {
    setRunning('cargo build');
    appendLine('hello', 'stdout');
    clearOutput();
    const s = getOutputState();
    expect(s.lines).toHaveLength(0);
    expect(s.exitCode).toBeNull();
    expect(s.cancelled).toBe(false);
    expect(s.command).toBeNull();
    // running stays true — clearOutput is safe mid-build
    expect(s.running).toBe(true);
  });
});
