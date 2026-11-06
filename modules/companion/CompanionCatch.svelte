<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { clampedX, isOver, secondsLeft, startCatch, step, TICK_MS, type CatchState } from './catch';
  import GameHeader from './GameHeader.svelte';
  import Mascot from './Mascot.svelte';
  import { companion } from './companion.svelte';
  import { t } from './i18n';

  const COUNTDOWN = 3;
  const COUNT_MS = 1000;

  let game = $state<CatchState>(startCatch());
  let count = $state(COUNTDOWN);
  let mascotX = $state(50);
  let timer: ReturnType<typeof setInterval> | undefined;

  const mascot = $derived(companion.mascot);

  function play(): void {
    timer = setInterval(() => {
      game = step(game, mascotX);
      if (isOver(game)) {
        clearInterval(timer);
        void companion.finish('catch', t('catch.result', { count: game.score }), game.score);
      }
    }, TICK_MS);
  }

  function move(event: MouseEvent): void {
    if (!(event.currentTarget instanceof HTMLElement)) return;
    const box = event.currentTarget.getBoundingClientRect();
    mascotX = clampedX(((event.clientX - box.left) / box.width) * 100);
  }

  onMount(() => {
    timer = setInterval(() => {
      count -= 1;
      if (count === 0) {
        clearInterval(timer);
        play();
      }
    }, COUNT_MS);
  });

  onDestroy(() => clearInterval(timer));
</script>

{#if mascot?.species}
  <div class="game">
    <GameHeader
      title={t('games.catch')}
      status={t('catch.status', { score: game.score, seconds: secondsLeft(game) })}
    />
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="field" onmousemove={move}>
      <div class="floor"></div>
      {#each game.items as item (item.id)}
        <span
          class="item"
          class:trap={item.trap}
          style:left="{item.x}%"
          style:top="{item.y}%"
          style:transform="rotate({item.turn}deg)"
        ></span>
      {/each}
      <div class="player" style:left="{mascotX}%">
        <Mascot species={mascot.species} mood={count > 0 ? 'idle' : 'walk'} wear={mascot.wear} size={140} />
      </div>
      {#if count > 0}
        <div class="countdown" aria-live="polite">
          {#key count}
            <span>{count}</span>
          {/key}
        </div>
      {/if}
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

  .field {
    position: relative;
    flex: 1;
    min-height: 0;
    border-radius: 16px;
    overflow: hidden;
    cursor: none;
    background: var(--surface);
    border: 1px solid var(--border);
  }

  .floor {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    height: 56px;
    background: var(--surface-sunken);
    border-top: 1px solid var(--border);
  }

  .item {
    position: absolute;
    width: 30px;
    height: 30px;
    margin-left: -15px;
    border-radius: 50%;
    background: #c98a4b;
    border: 2px solid #8a5a2b;
  }

  .trap {
    border-radius: 6px;
    background: #4a5568;
    border-color: var(--tint-red);
  }

  .player {
    position: absolute;
    bottom: 6px;
    margin-left: -70px;
    width: 140px;
    height: 140px;
    pointer-events: none;
  }

  .countdown {
    position: absolute;
    left: 0;
    right: 0;
    top: 0;
    bottom: 56px;
    display: flex;
    align-items: center;
    justify-content: center;
    pointer-events: none;
    font-size: 110px;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
  }

  .countdown span {
    animation: count 900ms ease-out both;
  }

  @keyframes count {
    0% {
      opacity: 0;
      transform: scale(0.6);
    }
    20% {
      opacity: 1;
      transform: scale(1);
    }
    80% {
      opacity: 1;
    }
    100% {
      opacity: 0.2;
    }
  }
</style>
