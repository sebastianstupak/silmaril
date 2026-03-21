<!-- Invisible resize handles for the frameless Tauri window.
     WebView2 intercepts WM_NCHITTEST before it reaches the parent HWND, so
     native edge-resize detection never fires. These divs post WM_NCLBUTTONDOWN
     with the appropriate hit code via window_start_resize, matching the same
     mechanism used for window drag. -->
<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';

  const isTauri = typeof window !== 'undefined' && !!(window as any).__TAURI_INTERNALS__;

  function resize(dir: string) {
    invoke('window_start_resize', { direction: dir }).catch(() => {});
  }
</script>

{#if isTauri}
  <div class="rh corner nw" onmousedown={() => resize('nw')} role="none"></div>
  <div class="rh corner ne" onmousedown={() => resize('ne')} role="none"></div>
  <div class="rh corner sw" onmousedown={() => resize('sw')} role="none"></div>
  <div class="rh corner se" onmousedown={() => resize('se')} role="none"></div>
  <div class="rh edge  n"  onmousedown={() => resize('n')}  role="none"></div>
  <div class="rh edge  s"  onmousedown={() => resize('s')}  role="none"></div>
  <div class="rh edge  w"  onmousedown={() => resize('w')}  role="none"></div>
  <div class="rh edge  e"  onmousedown={() => resize('e')}  role="none"></div>
{/if}

<style>
  .rh {
    position: fixed;
    z-index: 10000; /* above everything including the titlebar */
  }

  /* 4-pixel edge strips, inset by corner size */
  .edge.n { top: 0;      left: 8px;  right: 8px;  height: 4px; cursor: n-resize; }
  .edge.s { bottom: 0;   left: 8px;  right: 8px;  height: 4px; cursor: s-resize; }
  .edge.w { left: 0;     top: 8px;   bottom: 8px; width: 4px;  cursor: w-resize; }
  .edge.e { right: 0;    top: 8px;   bottom: 8px; width: 4px;  cursor: e-resize; }

  /* 8×8 corner squares */
  .corner.nw { top: 0;    left: 0;   width: 8px; height: 8px; cursor: nw-resize; }
  .corner.ne { top: 0;    right: 0;  width: 8px; height: 8px; cursor: ne-resize; }
  .corner.sw { bottom: 0; left: 0;   width: 8px; height: 8px; cursor: sw-resize; }
  .corner.se { bottom: 0; right: 0;  width: 8px; height: 8px; cursor: se-resize; }
</style>
