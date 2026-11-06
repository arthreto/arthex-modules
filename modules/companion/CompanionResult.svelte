<script lang="ts">
  import { Button } from '@arthex/sdk';
  import Mascot from './Mascot.svelte';
  import { companion } from './companion.svelte';
  import { t } from './i18n';

  const mascot = $derived(companion.mascot);
  const result = $derived(companion.result);
</script>

{#if mascot?.species && result}
  <div class="result">
    {#if result.gain.levelUp}
      <span class="level">{t('level', { level: mascot.level })}</span>
    {/if}
    <Mascot species={mascot.species} mood="happy" wear={mascot.wear} size={230} />
    <span class="title">{result.title}</span>
    <span class="gains">
      <span class="xp">+{result.gain.xp} XP</span>
      <span class="kibble">{t('kibble', { count: result.gain.kibble })}</span>
    </span>
    <span class="track"><span class="done" style:width="{(mascot.xp / mascot.xpNeeded) * 100}%"></span></span>
    <div class="actions">
      <Button onclick={() => (companion.view = 'home')}>{t('back')}</Button>
      <Button variant="primary" onclick={() => (companion.view = result.game)}>{t('replay')}</Button>
    </div>
  </div>
{/if}

<style>
  .result {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    border-radius: 16px;
    background: var(--surface);
    border: 1px solid var(--border);
  }

  .level {
    font-size: 28px;
    font-weight: 700;
    color: var(--tint-amber);
    animation: level 900ms cubic-bezier(0.2, 0.8, 0.2, 1);
  }

  @keyframes level {
    0% {
      transform: scale(0.6);
      opacity: 0;
    }
    60% {
      transform: scale(1.12);
      opacity: 1;
    }
  }

  .title {
    font-size: 20px;
    font-weight: 600;
  }

  .gains {
    display: flex;
    align-items: center;
    gap: 14px;
    font-size: 15px;
    font-weight: 600;
  }

  .xp {
    color: var(--accent);
  }

  .kibble {
    color: var(--tint-orange);
  }

  .track {
    display: block;
    width: 300px;
    height: 7px;
    border-radius: 4px;
    background: var(--surface-hover);
    overflow: hidden;
  }

  .done {
    display: block;
    height: 100%;
    border-radius: 4px;
    background: var(--accent);
  }

  .actions {
    display: flex;
    gap: 10px;
    margin-top: 4px;
  }
</style>
