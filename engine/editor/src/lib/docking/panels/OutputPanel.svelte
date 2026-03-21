<!-- engine/editor/src/lib/docking/panels/OutputPanel.svelte -->
<script lang="ts">
  import { t } from '$lib/i18n';
  import type { OutputState } from '$lib/stores/output';

  let { state, onRun, onCancel, onClear }: {
    state: OutputState;
    onRun: (command: string, args: string[]) => void;
    onCancel: () => void;
    onClear: () => void;
  } = $props();

  const COMMANDS = [
    { key: 'build',  cmd: 'cargo', args: ['build'],  labelKey: 'output.build'  },
    { key: 'test',   cmd: 'cargo', args: ['test'],   labelKey: 'output.test'   },
    { key: 'run',    cmd: 'cargo', args: ['run'],    labelKey: 'output.run'    },
    { key: 'clippy', cmd: 'cargo', args: ['clippy'], labelKey: 'output.clippy' },
  ] as const;

  let outputEl: HTMLDivElement;
  let userScrolledUp = false;

  function onScroll() {
    if (!outputEl) return;
    const { scrollTop, scrollHeight, clientHeight } = outputEl;
    userScrolledUp = scrollTop + clientHeight < scrollHeight - 20;
  }

  $effect(() => {
    // Scroll to bottom on new lines unless user scrolled up
    void state.lines.length;
    if (!userScrolledUp && outputEl) {
      outputEl.scrollTop = outputEl.scrollHeight;
    }
  });
</script>

<div class="output-panel">
  <!-- Button row -->
  <div class="toolbar">
    {#each COMMANDS as c}
      <button
        class="btn"
        disabled={state.running}
        onclick={() => onRun(c.cmd, [...c.args])}
      >{t(c.labelKey)}</button>
    {/each}
    {#if state.running}
      <button class="btn btn-cancel" onclick={onCancel}>{t('output.cancel')}</button>
    {/if}
    <button class="btn btn-clear" onclick={onClear}>{t('output.clear')}</button>
  </div>

  <!-- Output area -->
  <div
    class="output-area"
    role="log"
    aria-live="polite"
    bind:this={outputEl}
    onscroll={onScroll}
  >
    {#if state.lines.length === 0 && !state.running}
      <div class="placeholder">{t('output.empty')}</div>
    {:else}
      {#each state.lines as line, i (i)}
        <div class="output-line">
          {#each line.spans as span}
            <span
              style={[
                span.color ? `color:${span.color}` : '',
                span.bold ? 'font-weight:bold' : '',
              ].filter(Boolean).join(';')}
            >{span.text}</span>
          {/each}
        </div>
      {/each}
    {/if}
  </div>

  <!-- Status bar -->
  <div class="status-bar">
    {#if state.running}
      <span class="status-running">⟳ {state.command} {t('output.running')}</span>
    {:else if state.cancelled}
      <span class="status-cancelled">⊘ {t('output.cancelled')}</span>
    {:else if state.exitCode === 0 && state.command}
      <span class="status-ok">✓ {t('output.exit_ok')}</span>
    {:else if state.exitCode !== null && state.exitCode !== 0}
      <span class="status-err">✗ {t('output.exit_err')} (exit {state.exitCode})</span>
    {:else}
      <span class="status-idle">{state.command ?? ''}</span>
    {/if}
  </div>
</div>

<style>
  .output-panel {
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
    background: var(--color-bgPanel, #1e1e1e);
  }
  .toolbar {
    display: flex;
    gap: 4px;
    padding: 4px 8px;
    border-bottom: 1px solid var(--color-border, #333);
    flex-shrink: 0;
  }
  .btn {
    padding: 3px 10px;
    font-size: 12px;
    background: var(--color-bgHover, #2a2a2a);
    color: var(--color-text, #ccc);
    border: 1px solid var(--color-border, #333);
    border-radius: 3px;
    cursor: pointer;
  }
  .btn:hover:not(:disabled) { background: var(--color-accent, #569cd6); color: #fff; }
  .btn:disabled { opacity: 0.4; cursor: default; }
  .btn-cancel { border-color: #cc3e28; }
  .btn-clear { margin-left: auto; }
  .output-area {
    flex: 1;
    overflow-y: auto;
    padding: 6px 8px;
    font-family: Consolas, 'Courier New', monospace;
    font-size: 12px;
    line-height: 1.5;
  }
  .output-line { white-space: pre-wrap; word-break: break-all; }
  .placeholder { color: var(--color-textMuted, #666); font-style: italic; }
  .status-bar {
    padding: 3px 8px;
    font-size: 11px;
    border-top: 1px solid var(--color-border, #333);
    flex-shrink: 0;
    min-height: 22px;
  }
  .status-ok { color: #57a64a; }
  .status-err { color: #cc3e28; }
  .status-cancelled { color: #d7ba7d; }
  .status-running { color: var(--color-text, #ccc); }
  .status-idle { color: var(--color-textMuted, #666); }
</style>
