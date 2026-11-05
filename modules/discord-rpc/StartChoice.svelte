<script lang="ts">
  import { Icon } from '@arthex/sdk';
  import { t } from './i18n';
  import { ICON } from './identity';
  import { presence } from './presence.svelte';
  import type { Kind } from './template';

  const STARTS: { kind: Kind; icon: string }[] = [
    { kind: 'playing', icon: ICON },
    {
      kind: 'listening',
      icon: 'M3 14h3a2 2 0 0 1 2 2v3a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-7a9 9 0 0 1 18 0v7a2 2 0 0 1-2 2h-1a2 2 0 0 1-2-2v-3a2 2 0 0 1 2-2h3',
    },
    { kind: 'watching', icon: 'M3 7h18v12H3zM8 3l4 4 4-4' },
    {
      kind: 'competing',
      icon: 'M8 21h8M12 17v4M7 4h10v7a5 5 0 0 1-10 0zM7 6H4v2a3 3 0 0 0 3 3M17 6h3v2a3 3 0 0 1-3 3',
    },
  ];
</script>

<section class="choice">
  <h3>{t('nothingSelected')}</h3>
  <div class="starts">
    {#each STARTS as start (start.kind)}
      <button class="start" onclick={() => presence.create(start.kind)}>
        <span class="tile"><Icon path={start.icon} size={20} /></span>
        <span class="label">{t(`kinds.${start.kind}`)}</span>
      </button>
    {/each}
  </div>
</section>

<style>
  .choice {
    flex: 1;
    min-width: 0;
    padding: 20px;
    border-radius: 16px;
    background: var(--surface);
    border: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 20px;
  }

  h3 {
    margin: 0;
    font-size: 18px;
    font-weight: 600;
  }

  .starts {
    width: 100%;
    max-width: 440px;
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }

  .start {
    padding: 14px;
    border-radius: 14px;
    background: var(--surface);
    border: 1px solid var(--border);
    display: flex;
    align-items: center;
    gap: 12px;
    text-align: left;
    transition: background-color var(--motion-fast);
  }

  .start:hover {
    background: var(--surface-hover);
  }

  .tile {
    width: 40px;
    height: 40px;
    flex: none;
    box-sizing: border-box;
    border-radius: 12px;
    background: var(--surface-raised);
    border: 1px solid var(--border-strong);
    color: var(--tint-violet);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .label {
    font-size: 14px;
    font-weight: 600;
  }
</style>
