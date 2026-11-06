<script lang="ts">
  import { IconButton } from '@arthex/sdk';
  import type { Snippet } from 'svelte';
  import { companion } from './companion.svelte';
  import { t } from './i18n';

  interface Props {
    title: string;
    status?: string;
    children?: Snippet;
  }

  let { title, status, children }: Props = $props();
</script>

<header>
  <IconButton icon="chevronLeft" label={t('back')} onclick={() => (companion.view = 'home')} />
  <h2>{title}</h2>
  {@render children?.()}
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
