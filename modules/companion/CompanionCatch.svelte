<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { clampedX, isOver, secondsLeft, startCatch, step, type CatchState } from './catch';
  import GameHeader from './GameHeader.svelte';
  import Mascot from './Mascot.svelte';
  import { companion } from './companion.svelte';
  import { t } from './i18n';

  const COUNTDOWN = 3;
  const COUNT_MS = 1000;
  /** After a pause, such as the overlay closing, the game goes on where it was. */
  const LONGEST_FRAME_MS = 100;

  let game = $state.raw<CatchState>(startCatch());
  let count = $state(COUNTDOWN);
  let mascotX = $state(50);
  let pointerX = 50;
  let countdown: ReturnType<typeof setInterval> | undefined;
  let frame = 0;
  let lastFrame: number | undefined;

  const mascot = $derived(companion.mascot);

  function advance(now: number): void {
    frame = requestAnimationFrame(advance);
    const ms = Math.min(now - (lastFrame ?? now), LONGEST_FRAME_MS);
    lastFrame = now;
    mascotX = pointerX;
    if (count > 0) return;
    game = step(game, pointerX, ms);
    if (isOver(game)) {
      cancelAnimationFrame(frame);
      void companion.finish('catch', t('catch.result', { count: game.score }), game.score);
    }
  }

  function move(event: MouseEvent): void {
    if (!(event.currentTarget instanceof HTMLElement)) return;
    const box = event.currentTarget.getBoundingClientRect();
    pointerX = clampedX(((event.clientX - box.left) / box.width) * 100);
  }

  onMount(() => {
    frame = requestAnimationFrame(advance);
    countdown = setInterval(() => {
      count -= 1;
      if (count === 0) clearInterval(countdown);
    }, COUNT_MS);
  });

  onDestroy(() => {
    clearInterval(countdown);
    cancelAnimationFrame(frame);
  });
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
          style:transform="translate({item.x}cqw, {item.y}cqh) rotate({item.turn}deg)"
        ></span>
      {/each}
      <div class="player" style:transform="translateX({mascotX}cqw)">
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
    /* Moving by transforms alone, so that nothing is laid out again under the blur of the overlay. */
    container-type: size;
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
    left: 0;
    top: 0;
    width: 30px;
    height: 30px;
    margin-left: -15px;
    will-change: transform;
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
    left: 0;
    bottom: 6px;
    margin-left: -70px;
    will-change: transform;
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
