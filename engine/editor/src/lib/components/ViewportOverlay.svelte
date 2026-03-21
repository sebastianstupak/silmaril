<script lang="ts">
  // ViewportOverlay renders in a separate transparent WebView2 window that sits
  // above the native Vulkan child window (the sandwich architecture).
  // This WebView2 is created by create_native_viewport with overlay=true and
  // receives ?overlay=viewport in its URL so App.svelte routes here.
  //
  // It provides a pointer-events-none transparent container for future HUD
  // elements (gizmos, selection outlines, tool overlays) that need to appear
  // above the Vulkan framebuffer without blocking its input events.
  //
  // Gizmo mouse input (hit test, drag, drag end) and keyboard shortcuts (W/E/R)
  // are handled in ViewportPanel.svelte, which owns all mouse/keyboard handlers
  // for the viewport.  The gizmo IPC wrappers live in api.ts:
  //   gizmoHitTest, gizmoDrag, gizmoDragEnd, setGizmoMode.
</script>

<div class="viewport-overlay" aria-hidden="true"></div>

<style>
  .viewport-overlay {
    position: fixed;
    inset: 0;
    pointer-events: none;
    background: transparent;
  }
</style>
