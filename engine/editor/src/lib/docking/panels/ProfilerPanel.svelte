<script lang="ts">
  import { t } from '$lib/i18n';
</script>

<div class="profiler-placeholder">
  <p>{t('panel.profiler')}</p>
</div>

<style>
  .profiler-placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--color-textDim, #666);
    font-style: italic;
    background: var(--color-bgPanel, #252525);
  }
</style>
