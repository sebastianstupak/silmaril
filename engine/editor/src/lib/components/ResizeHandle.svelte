<script lang="ts">
  interface Props {
    direction: 'horizontal' | 'vertical';
    onResize: (delta: number) => void;
  }

  let { direction, onResize }: Props = $props();
  let dragging = $state(false);

  function onMouseDown(e: MouseEvent) {
    e.preventDefault();
    dragging = true;
    const startX = e.clientX;
    const startY = e.clientY;

    function onMouseMove(e: MouseEvent) {
      const delta = direction === 'horizontal'
        ? e.clientX - startX
        : e.clientY - startY;
      if (delta !== 0) {
        onResize(delta);
      }
      // Reset start for continuous delta
      Object.assign(e, { clientX: e.clientX, clientY: e.clientY });
    }

    // Use a simpler approach: track cumulative from mousedown
    let lastX = startX;
    let lastY = startY;

    function onMove(e: MouseEvent) {
      const dx = e.clientX - lastX;
      const dy = e.clientY - lastY;
      lastX = e.clientX;
      lastY = e.clientY;
      onResize(direction === 'horizontal' ? dx : dy);
    }

    function onMouseUp() {
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
  class="resize-handle {direction}"
  class:dragging
  onmousedown={onMouseDown}
></div>

<style>
  .resize-handle {
    flex-shrink: 0;
    background: transparent;
    transition: background 0.15s;
    z-index: 10;
  }
  .resize-handle:hover, .resize-handle.dragging {
    background: var(--color-accent, #007acc);
  }
  .horizontal {
    width: 4px;
    cursor: col-resize;
  }
  .vertical {
    height: 4px;
    cursor: row-resize;
  }
</style>
