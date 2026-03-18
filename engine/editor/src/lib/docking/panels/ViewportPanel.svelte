<script lang="ts">
  import { t } from '$lib/i18n';
</script>

<div class="viewport-placeholder">
  <p>{t('placeholder.viewport')}</p>
</div>

<style>
  .viewport-placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--color-textDim, #666);
  }
</style>
