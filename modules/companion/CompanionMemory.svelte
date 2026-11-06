<script lang="ts">
  import { Icon } from '@arthex/sdk';
  import { onDestroy } from 'svelte';
  import GameHeader from './GameHeader.svelte';
  import Mascot from './Mascot.svelte';
  import { companion } from './companion.svelte';
  import { t } from './i18n';
  import { dealCards, flip, isWon, PICTURES, settle } from './memory';

  const SEEN_MS = 550;

  let game = $state(dealCards());
  let timer: ReturnType<typeof setTimeout> | undefined;

  const mascot = $derived(companion.mascot);

  function turn(card: number): void {
    game = flip(game, card);
    if (game.open.length < 2) return;
    timer = setTimeout(() => {
      game = settle(game);
      if (isWon(game)) void companion.finish('memory', t('memory.moves', { count: game.moves }));
    }, SEEN_MS);
  }

  onDestroy(() => clearTimeout(timer));
</script>

{#if mascot?.species}
  <div class="game">
    <GameHeader title={t('games.memory')} status={t('memory.moves', { count: game.moves })} />
    <div class="table">
      <div class="cards">
        {#each game.cards as picture, card (card)}
          {@const done = game.done.includes(card)}
          {@const open = done || game.open.includes(card)}
          {@const shown = PICTURES[picture]}
          <button
            type="button"
            class="card"
            class:done
            aria-label={t('memory.card', { number: card + 1 })}
            onclick={() => turn(card)}
          >
            {#if open && shown}
              <span class="picture" style:color="var(--tint-{shown.tint})">
                <Icon path={shown.icon} size={46} stroke={1.6} />
              </span>
            {:else}
              <span class="back"></span>
            {/if}
          </button>
        {/each}
      </div>
      <div class="watcher">
        <Mascot
          species={mascot.species}
          mood={game.done.length > 2 ? 'happy' : 'idle'}
          wear={mascot.wear}
          size={220}
        />
      </div>
    </div>
  </div>
{/if}

<style>
  .game {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .table {
    flex: 1;
    min-height: 0;
    display: flex;
    gap: 14px;
  }

  .cards {
    flex: 1;
    min-width: 0;
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    grid-template-rows: repeat(3, minmax(0, 1fr));
    gap: 10px;
  }

  .card {
    border-radius: 14px;
    display: flex;
    align-items: center;
    justify-content: center;
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

  .done {
    opacity: 0.45;
  }

  .picture {
    display: flex;
    animation: flip 220ms ease-out;
  }

  @keyframes flip {
    from {
      transform: rotateY(90deg);
    }
  }

  .back {
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: var(--surface-hover);
  }

  .watcher {
    width: 240px;
    flex: none;
    display: flex;
    align-items: flex-end;
    justify-content: center;
  }
</style>
