<script>
  // Silmaril Editor — Panel Shell
  // All panels are loaded from modules via the plugin system
</script>

<main class="editor-shell">
  <div class="toolbar">
    <span class="title">Silmaril Editor</span>
  </div>
  <div class="panels">
    <p>No panels loaded. Open a project to load module panels.</p>
  </div>
</main>

<style>
  .editor-shell {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: #1e1e1e;
    color: #cccccc;
    font-family: system-ui, -apple-system, sans-serif;
  }
  .toolbar {
    height: 40px;
    display: flex;
    align-items: center;
    padding: 0 16px;
    background: #2d2d2d;
    border-bottom: 1px solid #404040;
  }
  .title {
    font-weight: 600;
    font-size: 14px;
  }
  .panels {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    color: #666;
  }
</style>
