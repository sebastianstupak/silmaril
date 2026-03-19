<script lang="ts">
  import { t } from '$lib/i18n';
</script>

<div class="assets-placeholder">
  <p>{t('panel.assets')}</p>
</div>

<style>
  .assets-placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--color-textDim, #666);
    font-style: italic;
    background: var(--color-bgPanel, #252525);
  }
</style>
