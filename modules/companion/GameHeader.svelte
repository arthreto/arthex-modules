<script lang="ts">
  import { IconButton } from '@arthex/sdk';
  import { companion } from './companion.svelte';
  import { t } from './i18n';

  interface Props {
    title: string;
    status?: string;
    onback?: () => void;
  }

  let { title, status, onback }: Props = $props();

  function back(): void {
    onback?.();
    companion.view = 'home';
  }
</script>

<header>
  <IconButton icon="chevronLeft" label={t('back')} onclick={back} />
  <h2>{title}</h2>
  {#if status}
    <span class="status">{status}</span>
  {/if}
</header>

<style>
  header {
    flex: none;
    min-height: 36px;
    display: flex;
    align-items: center;
    gap: 10px;
  }

  h2 {
    flex: 1;
    min-width: 0;
    font-size: 20px;
    font-weight: 600;
  }

  .status {
    font-size: 14px;
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
  }
</style>
