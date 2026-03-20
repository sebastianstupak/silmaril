<script lang="ts">
  // Module-level mutex: only one DockSplitter instance may be dragging at a time.
  // Module-level variables in .svelte files are shared across all instances
  // in the same JS module scope, which is the intended behavior here.
  let activeSplitter: symbol | null = null;

  interface Props {
    direction: 'horizontal' | 'vertical';
    onResize: (deltaPx: number) => void;
  }

  let { direction, onResize }: Props = $props();
  let dragging = $state(false);

  function onMouseDown(e: MouseEvent) {
    // Guard: ignore if another splitter is already dragging
    if (activeSplitter !== null) return;

    e.preventDefault();
    e.stopPropagation();

    const id = Symbol();
    activeSplitter = id;
    dragging = true;

    let lastX = e.clientX;
    let lastY = e.clientY;

    function onMove(e: MouseEvent) {
      const dx = e.clientX - lastX;
      const dy = e.clientY - lastY;
      lastX = e.clientX;
      lastY = e.clientY;
      onResize(direction === 'horizontal' ? dx : dy);
    }

    function onMouseUp() {
      if (activeSplitter === id) activeSplitter = null;
      dragging = false;
      window.removeEventListener('mousemove', onMove);
      window.removeEventListener('mouseup', onMouseUp);
      document.body.style.cursor = '';
      document.body.style.userSelect = '';
    }

    document.body.style.cursor = direction === 'horizontal' ? 'col-resize' : 'row-resize';
    document.body.style.userSelect = 'none';
    window.addEventListener('mousemove', onMove);
    window.addEventListener('mouseup', onMouseUp);
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="dock-splitter {direction}"
  class:dragging
  onmousedown={onMouseDown}
></div>

<style>
  .dock-splitter {
    flex-shrink: 0;
    background: var(--color-bg, #1e1e1e);
    z-index: 10;
    position: relative;
    box-sizing: border-box;
    transition: border-color 0.15s;
  }
  .dock-splitter.horizontal {
    width: 8px;
    cursor: col-resize;
    border-left: 1px solid var(--color-border, #404040);
    border-right: 1px solid var(--color-border, #404040);
  }
  .dock-splitter.vertical {
    height: 8px;
    cursor: row-resize;
    border-top: 1px solid var(--color-border, #404040);
    border-bottom: 1px solid var(--color-border, #404040);
  }
  .dock-splitter:hover,
  .dock-splitter.dragging {
    border-color: var(--color-accent, #007acc);
  }
</style>
