<script lang="ts">
  import { navigate } from '@arthex/sdk';
  import { onMount } from 'svelte';
  import Mascot from './Mascot.svelte';
  import { companion } from './companion.svelte';
  import { t } from './i18n';
  import { ID } from './identity';

  const mascot = $derived(companion.mascot);

  onMount(() => {
    void companion.load();
  });
</script>

{#if mascot}
  <button class="card" onclick={() => navigate(ID)}>
    {#if mascot.species}
      <Mascot species={mascot.species} wear={mascot.wear} size={76} />
    {/if}
    <span class="content">
      <span class="head">
        {#if mascot.species}
          <span>{mascot.name} <span class="level">· {t('level', { level: mascot.level })}</span></span>
        {:else}
          <span>{t('name')}</span>
        {/if}
        <span class="link">{t('play')} →</span>
      </span>
      {#if mascot.species}
        <span class="track"
          ><span class="done" style:width="{(mascot.xp / mascot.xpNeeded) * 100}%"></span></span
        >
      {/if}
    </span>
  </button>
{/if}

<style>
  .card {
    width: 100%;
    padding: 10px 16px 10px 8px;
    border-radius: 14px;
    display: flex;
    align-items: center;
    gap: 8px;
    background: var(--surface);
    border: 1px solid var(--border);
    transition:
      background-color var(--motion-fast),
      transform var(--motion-fast);
  }

  .card:hover {
    background: var(--surface-hover);
  }

  .card:active {
    transform: scale(0.985);
  }

  .content {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    font-size: 13.5px;
    font-weight: 600;
  }

  .level {
    font-size: 12px;
    font-weight: 400;
    color: var(--text-subtle);
  }

  .link {
    font-size: 12.5px;
    font-weight: 500;
    color: var(--accent);
  }

  .track {
    display: block;
    height: 6px;
    border-radius: 3px;
    background: var(--surface-hover);
    overflow: hidden;
  }

  .done {
    display: block;
    height: 100%;
    border-radius: 3px;
    background: var(--accent);
  }
</style>
