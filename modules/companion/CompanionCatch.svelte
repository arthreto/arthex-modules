<script lang="ts">
  import { Button } from '@arthex/sdk';
  import { onDestroy } from 'svelte';
  import { clampedX, isOver, secondsLeft, startCatch, step, TICK_MS, type CatchState } from './catch';
  import GameHeader from './GameHeader.svelte';
  import Mascot from './Mascot.svelte';
  import { companion } from './companion.svelte';
  import { t } from './i18n';

  let game = $state<CatchState>(startCatch());
  let playing = $state(false);
  let mascotX = $state(50);
  let timer: ReturnType<typeof setInterval> | undefined;

  const mascot = $derived(companion.mascot);

  function start(): void {
    game = startCatch();
    playing = true;
    timer = setInterval(() => {
      game = step(game, mascotX);
      if (isOver(game)) {
        stop();
        void companion.finish('catch', t('catch.result', { count: game.score }), game.score);
      }
    }, TICK_MS);
  }

  function stop(): void {
    clearInterval(timer);
    playing = false;
  }

  function move(event: MouseEvent): void {
    if (!(event.currentTarget instanceof HTMLElement)) return;
    const box = event.currentTarget.getBoundingClientRect();
    mascotX = clampedX(((event.clientX - box.left) / box.width) * 100);
  }

  onDestroy(stop);
</script>

{#if mascot?.species}
  <div class="game">
    <GameHeader
      title={t('games.catch')}
      status={t('catch.status', { score: game.score, seconds: secondsLeft(game) })}
      onback={stop}
    />
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="field" class:playing onmousemove={move}>
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
        <Mascot species={mascot.species} mood={playing ? 'walk' : 'idle'} wear={mascot.wear} size={140} />
      </div>
      {#if !playing}
        <div class="start"><Button variant="primary" onclick={start}>{t('play')}</Button></div>
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
    background: var(--surface);
    border: 1px solid var(--border);
  }

  .playing {
    cursor: none;
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

  .start {
    position: absolute;
    left: 0;
    right: 0;
    top: 16px;
    display: flex;
    justify-content: center;
  }

  .start :global(.button) {
    height: 44px;
    padding: 0 26px;
    font-size: 14px;
    cursor: pointer;
  }
</style>
