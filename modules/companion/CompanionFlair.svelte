<script lang="ts">
  import { Button, getLocale, TextField } from '@arthex/sdk';
  import { onMount } from 'svelte';
  import GameHeader from './GameHeader.svelte';
  import Mascot from './Mascot.svelte';
  import { companion } from './companion.svelte';
  import { t } from './i18n';
  import { barWidth, heatOf, isExcited, warmestFirst, type Heat } from './words';

  const HEAT_TINT: Record<Heat, string> = {
    hot: 'var(--heat-hot)',
    warm: 'var(--tint-orange)',
    cool: 'var(--tint-sky)',
    cold: 'var(--text-subtle)',
  };

  const mascot = $derived(companion.mascot);
  let input = $state('');

  const words = $derived(companion.round ?? []);
  const sorted = $derived(warmestFirst(words));
  const best = $derived(sorted[0]?.temperature ?? null);

  const degrees = (temperature: number) =>
    `${new Intl.NumberFormat(getLocale(), { minimumFractionDigits: 1, maximumFractionDigits: 1 }).format(temperature)} °C`;

  function submit(event: SubmitEvent): void {
    event.preventDefault();
    void companion.guess(input);
    input = '';
  }

  onMount(() => {
    void companion.startRound();
  });
</script>

{#if mascot?.species}
  <div class="game">
    <GameHeader title={t('games.flair')} status={t('flair.tries', { count: words.length })} />
    <div class="play">
      <div class="side">
        {#if best !== null}
          <span class="best" style:color={HEAT_TINT[heatOf(best)]}>{degrees(best)}</span>
        {/if}
        <Mascot
          species={mascot.species}
          mood={isExcited(best) ? 'happy' : 'sniff'}
          wear={mascot.wear}
          size={220}
        />
      </div>
      <div class="main">
        <form onsubmit={submit}>
          <span class="word">
            <TextField
              label={t('flair.word')}
              hideLabel
              placeholder={t('flair.placeholder')}
              autocomplete="off"
              spellcheck={false}
              bind:value={input}
            />
          </span>
          <Button type="submit" variant="primary" disabled={!companion.round}>{t('flair.try')}</Button>
          <Button disabled={!companion.round} onclick={() => void companion.giveUp()}
            >{t('flair.giveUp')}</Button
          >
        </form>
        {#if companion.roundMessage}
          <span class="error">{companion.roundMessage}</span>
        {/if}
        <div class="list">
          {#each sorted as tried, index (tried.word)}
            <div class="row" class:first={index === 0}>
              <span class="name">{tried.word}</span>
              <span class="degrees" style:color={HEAT_TINT[heatOf(tried.temperature)]}
                >{degrees(tried.temperature)}</span
              >
              <span class="track">
                <span
                  class="done"
                  style:width="{barWidth(tried.temperature)}%"
                  style:background={HEAT_TINT[heatOf(tried.temperature)]}
                ></span>
              </span>
            </div>
          {/each}
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  .game {
    --heat-hot: #ff7a59;
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  :global(:root[data-theme='light']) .game {
    --heat-hot: #c2410c;
  }

  .play {
    flex: 1;
    min-height: 0;
    display: flex;
    gap: 14px;
  }

  .side {
    width: 240px;
    flex: none;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: flex-end;
    gap: 6px;
  }

  .best {
    font-size: 30px;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
  }

  .main {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  form {
    flex: none;
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .word {
    flex: 1;
    min-width: 0;
  }

  form :global(.button) {
    height: 44px;
    padding: 0 22px;
    font-size: 14px;
  }

  .error {
    font-size: 12.5px;
    color: var(--danger-text);
  }

  .list {
    flex: 1;
    min-height: 0;
    padding: 4px 16px;
    overflow: auto;
    border-radius: 14px;
    background: var(--surface);
    border: 1px solid var(--border);
  }

  .row {
    height: 42px;
    display: flex;
    align-items: center;
    gap: 12px;
    border-top: 1px solid var(--border);
  }

  .first {
    border-top: none;
  }

  .name {
    width: 130px;
    flex: none;
    font-size: 14px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .first .name {
    font-weight: 600;
  }

  .degrees {
    width: 70px;
    flex: none;
    text-align: right;
    font-size: 13.5px;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }

  .track {
    flex: 1;
    min-width: 0;
    display: block;
    height: 7px;
    border-radius: 4px;
    background: var(--surface-hover);
    overflow: hidden;
  }

  .done {
    display: block;
    height: 100%;
    border-radius: 4px;
  }
</style>
