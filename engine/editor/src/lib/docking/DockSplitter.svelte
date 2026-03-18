<script lang="ts">
  interface Props {
    direction: 'horizontal' | 'vertical';
    onResize: (deltaPx: number) => void;
  }

  let { direction, onResize }: Props = $props();
  let dragging = $state(false);

  function onMouseDown(e: MouseEvent) {
    e.preventDefault();
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
    background: transparent;
    transition: background 0.15s;
    z-index: 10;
    position: relative;
  }
  .dock-splitter:hover, .dock-splitter.dragging {
    background: var(--color-accent, #007acc);
  }
  .dock-splitter.horizontal {
    width: 4px;
    cursor: col-resize;
  }
  .dock-splitter.vertical {
    height: 4px;
    cursor: row-resize;
  }
</style>
