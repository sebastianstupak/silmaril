<script>
  // Loads Tier 3 JS bundle panels in a sandboxed iframe
  export let bundlePath = '';
</script>

<div class="plugin-frame">
  {#if bundlePath}
    <iframe
      src={bundlePath}
      sandbox="allow-scripts"
      title="Plugin Panel"
    />
  {:else}
    <p>No plugin bundle specified</p>
  {/if}
</div>

<style>
  .plugin-frame {
    width: 100%;
    height: 100%;
  }
  iframe {
    width: 100%;
    height: 100%;
    border: none;
    background: #1e1e1e;
  }
</style>
