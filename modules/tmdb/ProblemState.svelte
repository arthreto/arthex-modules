<script lang="ts">
  import { Button, EmptyState } from '@arthex/sdk';
  import { t } from './i18n';
  import { ICON } from './identity';
  import type { Problem } from './library';

  let { problem, onretry }: { problem: Problem; onretry: () => void } = $props();
</script>

<div class="center">
  <EmptyState icon={ICON} tint="var(--tint-sky)" title={t(problem === 'offline' ? 'offline' : 'failed')}>
    <Button onclick={onretry}>{t('retry')}</Button>
  </EmptyState>
</div>

<style>
  .center {
    flex: 1;
    min-height: 0;
    display: flex;
    align-items: center;
    justify-content: center;
  }
</style>
