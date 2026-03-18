<script lang="ts">
  import { t } from '$lib/i18n';
  import { onMount, tick } from 'svelte';
  import {
    type LogEntry,
    getLogs,
    clearLogs as clearLogStore,
    subscribeConsole,
  } from '$lib/stores/console';

  let logs: LogEntry[] = $state([]);
  let filter = $state('');
  let showInfo = $state(true);
  let showWarn = $state(true);
  let showError = $state(true);
  let showDebug = $state(true);
  let scrollContainer: HTMLElement | undefined = $state(undefined);

  const levelColors: Record<LogEntry['level'], string> = {
    info: 'var(--console-info, #ccc)',
    warn: 'var(--console-warn, #ff9800)',
    error: 'var(--console-error, #f44336)',
    debug: 'var(--console-debug, #666)',
  };

  const levelBgColors: Record<LogEntry['level'], string> = {
    info: 'var(--console-info-bg, rgba(204,204,204,0.12))',
    warn: 'var(--console-warn-bg, rgba(255,152,0,0.15))',
    error: 'var(--console-error-bg, rgba(244,67,54,0.15))',
    debug: 'var(--console-debug-bg, rgba(102,102,102,0.12))',
  };

  let filteredLogs = $derived.by(() => {
    const lowerFilter = filter.toLowerCase();
    return logs.filter((entry) => {
      if (entry.level === 'info' && !showInfo) return false;
      if (entry.level === 'warn' && !showWarn) return false;
      if (entry.level === 'error' && !showError) return false;
      if (entry.level === 'debug' && !showDebug) return false;
      if (lowerFilter && !entry.message.toLowerCase().includes(lowerFilter)) return false;
      return true;
    });
  });

  let counts = $derived.by(() => {
    const c = { info: 0, warn: 0, error: 0, debug: 0 };
    for (const entry of logs) {
      c[entry.level]++;
    }
    return c;
  });

  function formatTimestamp(ts: string): string {
    // Timestamps are already formatted as HH:MM:SS.mmm by the log store
    if (/^\d{2}:\d{2}:\d{2}/.test(ts)) return ts;
    // Fallback for ISO strings
    try {
      const d = new Date(ts);
      return d.toLocaleTimeString('en-US', { hour12: false, hour: '2-digit', minute: '2-digit', second: '2-digit' })
        + '.' + String(d.getMilliseconds()).padStart(3, '0');
    } catch {
      return ts;
    }
  }

  function clearLogs() {
    clearLogStore();
    logs = [];
  }

  async function scrollToBottom() {
    await tick();
    if (scrollContainer) {
      scrollContainer.scrollTop = scrollContainer.scrollHeight;
    }
  }

  // Auto-scroll when new logs arrive
  $effect(() => {
    // Track logs length to trigger on new entries
    if (logs.length > 0) {
      scrollToBottom();
    }
  });

  onMount(() => {
    logs = getLogs();
    const unsub = subscribeConsole(() => {
      logs = getLogs();
      scrollToBottom();
    });
    return unsub;
  });
</script>

<div class="console-panel">
  <div class="console-toolbar">
    <button
      class="console-btn clear-btn"
      onclick={clearLogs}
      title={t('console.clear')}
    >
      {t('console.clear')}
    </button>

    <span class="toolbar-divider"></span>

    <button
      class="console-btn filter-toggle"
      class:active={showInfo}
      style="--toggle-color: {levelColors.info}"
      onclick={() => showInfo = !showInfo}
    >
      {t('console.info')}
      <span class="count-badge">{counts.info}</span>
    </button>
    <button
      class="console-btn filter-toggle"
      class:active={showWarn}
      style="--toggle-color: {levelColors.warn}"
      onclick={() => showWarn = !showWarn}
    >
      {t('console.warn')}
      <span class="count-badge">{counts.warn}</span>
    </button>
    <button
      class="console-btn filter-toggle"
      class:active={showError}
      style="--toggle-color: {levelColors.error}"
      onclick={() => showError = !showError}
    >
      {t('console.error')}
      <span class="count-badge">{counts.error}</span>
    </button>
    <button
      class="console-btn filter-toggle"
      class:active={showDebug}
      style="--toggle-color: {levelColors.debug}"
      onclick={() => showDebug = !showDebug}
    >
      {t('console.debug')}
      <span class="count-badge">{counts.debug}</span>
    </button>

    <span class="toolbar-spacer"></span>

    <input
      type="text"
      class="console-search"
      placeholder={t('console.filter')}
      bind:value={filter}
    />
  </div>

  <div class="console-log-list" bind:this={scrollContainer}>
    {#if filteredLogs.length === 0}
      <p class="console-empty">{t('console.no_logs')}</p>
    {:else}
      {#each filteredLogs as entry (entry.timestamp + entry.message)}
        <div
          class="log-row"
          style="--row-color: {levelColors[entry.level]}"
        >
          <span class="log-timestamp">{formatTimestamp(entry.timestamp)}</span>
          <span
            class="log-level-badge"
            style="background: {levelBgColors[entry.level]}; color: {levelColors[entry.level]}"
          >
            {entry.level.toUpperCase()}
          </span>
          <span class="log-message">{entry.message}</span>
        </div>
      {/each}
    {/if}
  </div>
</div>

<style>
  .console-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    font-family: var(--font-mono, 'Consolas', 'Courier New', monospace);
    font-size: 12px;
    overflow: hidden;
  }

  .console-toolbar {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 4px 8px;
    background: var(--color-bgHeader, #2d2d2d);
    border-bottom: 1px solid var(--color-border, #404040);
    flex-shrink: 0;
  }

  .console-btn {
    background: none;
    border: 1px solid transparent;
    color: var(--color-textMuted, #999);
    cursor: pointer;
    padding: 2px 6px;
    border-radius: 3px;
    font-size: 11px;
    font-family: inherit;
    display: flex;
    align-items: center;
    gap: 4px;
    white-space: nowrap;
  }

  .console-btn:hover {
    background: var(--color-bgPanel, #252525);
    border-color: var(--color-border, #404040);
    color: var(--color-text, #ccc);
  }

  .clear-btn {
    color: var(--color-textDim, #666);
  }

  .filter-toggle {
    color: var(--color-textDim, #666);
  }

  .filter-toggle.active {
    color: var(--toggle-color);
    border-color: color-mix(in srgb, var(--toggle-color) 30%, transparent);
    background: color-mix(in srgb, var(--toggle-color) 8%, transparent);
  }

  .count-badge {
    font-size: 10px;
    padding: 0 4px;
    border-radius: 8px;
    background: var(--color-bg, #1e1e1e);
    color: var(--color-textDim, #666);
    min-width: 16px;
    text-align: center;
    line-height: 16px;
  }

  .filter-toggle.active .count-badge {
    color: var(--toggle-color);
  }

  .toolbar-divider {
    width: 1px;
    height: 16px;
    background: var(--color-border, #404040);
    flex-shrink: 0;
    margin: 0 2px;
  }

  .toolbar-spacer {
    flex: 1;
  }

  .console-search {
    background: var(--color-bg, #1e1e1e);
    border: 1px solid var(--color-border, #404040);
    color: var(--color-text, #ccc);
    padding: 2px 8px;
    border-radius: 3px;
    font-size: 11px;
    font-family: inherit;
    width: 180px;
    outline: none;
  }

  .console-search:focus {
    border-color: var(--color-accent, #007acc);
  }

  .console-search::placeholder {
    color: var(--color-textDim, #666);
  }

  .console-log-list {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
  }

  .console-empty {
    color: var(--color-textDim, #666);
    font-style: italic;
    padding: 12px;
    text-align: center;
  }

  .log-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 1px 8px;
    height: 22px;
    border-bottom: 1px solid color-mix(in srgb, var(--color-border, #404040) 30%, transparent);
    color: var(--row-color);
  }

  .log-row:hover {
    background: var(--color-bg, #1e1e1e);
  }

  .log-timestamp {
    color: var(--color-textDim, #666);
    font-size: 11px;
    flex-shrink: 0;
    width: 85px;
  }

  .log-level-badge {
    font-size: 9px;
    font-weight: 700;
    padding: 0 4px;
    border-radius: 2px;
    flex-shrink: 0;
    width: 40px;
    text-align: center;
    letter-spacing: 0.5px;
    line-height: 16px;
  }

  .log-message {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
    min-width: 0;
  }
</style>
