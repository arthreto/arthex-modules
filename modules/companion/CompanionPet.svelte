<script lang="ts">
  import { onDestroy } from 'svelte';
  import GameHeader from './GameHeader.svelte';
  import Mascot from './Mascot.svelte';
  import { companion } from './companion.svelte';
  import { t } from './i18n';

  const HAPPY_MS = 900;
  const RESULT_AFTER_MS = 500;

  const mascot = $derived(companion.mascot);
  let happy = $state(false);
  let full = $state(false);
  let hearts = $state<number[]>([]);
  let happyTimer: ReturnType<typeof setTimeout> | undefined;
  let resultTimer: ReturnType<typeof setTimeout> | undefined;

  const count = $derived(mascot ? (full ? mascot.petGoal : Math.min(mascot.pets, mascot.petGoal)) : 0);

  async function stroke(): Promise<void> {
    if (full) return;
    happy = true;
    hearts = [...hearts.slice(-3), Date.now()];
    clearTimeout(happyTimer);
    happyTimer = setTimeout(() => {
      happy = false;
      hearts = [];
    }, HAPPY_MS);
    const gain = await companion.pet();
    if (gain) {
      full = true;
      resultTimer = setTimeout(() => companion.showResult('pet', t('pet.result'), gain), RESULT_AFTER_MS);
    }
  }

  onDestroy(() => {
    clearTimeout(happyTimer);
    clearTimeout(resultTimer);
  });
</script>

{#if mascot?.species}
  <div class="game">
    <GameHeader title={t('games.pet')} status="{count} / {mascot.petGoal}" />
    <div class="stage">
      <button class="pet" aria-label={t('pet.action', { name: mascot.name })} onclick={() => void stroke()}>
        <Mascot species={mascot.species} mood={happy ? 'happy' : 'idle'} wear={mascot.wear} size={300} />
      </button>
      {#each hearts as heart, index (heart)}
        <span class="heart" style:left="{42 + index * 5}%">♥</span>
      {/each}
      <span
        class="track"
        role="progressbar"
        aria-label={t('pet.progress')}
        aria-valuenow={count}
        aria-valuemin={0}
        aria-valuemax={mascot.petGoal}
      >
        <span class="done" style:width="{(count / mascot.petGoal) * 100}%"></span>
      </span>
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

  .stage {
    position: relative;
    flex: 1;
    min-height: 0;
    border-radius: 16px;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
    background: var(--surface);
    border: 1px solid var(--border);
  }

  .pet {
    width: 300px;
    height: 300px;
  }

  .heart {
    position: absolute;
    top: 30%;
    font-size: 15px;
    font-weight: 700;
    color: var(--tint-pink);
    pointer-events: none;
    animation: rise 700ms ease-out forwards;
  }

  @keyframes rise {
    0% {
      opacity: 0;
      transform: translateY(8px) scale(0.8);
    }
    25% {
      opacity: 1;
    }
    100% {
      opacity: 0;
      transform: translateY(-46px);
    }
  }

  .track {
    display: block;
    width: 320px;
    height: 8px;
    border-radius: 4px;
    background: var(--surface-hover);
    overflow: hidden;
  }

  .done {
    display: block;
    height: 100%;
    background: var(--tint-pink);
    transition: width 120ms linear;
  }
</style>
