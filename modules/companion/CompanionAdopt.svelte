<script lang="ts">
  import { Button, TextField } from '@arthex/sdk';
  import Mascot from './Mascot.svelte';
  import { companion, type Species } from './companion.svelte';
  import { t } from './i18n';

  const ANIMALS: Species[] = ['fox', 'dog', 'cat', 'wolf'];

  let chosen = $state<Species>('fox');
  let name = $state('');

  function adopt(event: SubmitEvent): void {
    event.preventDefault();
    if (name.trim()) void companion.adopt(chosen, name);
  }
</script>

<div class="adopt">
  <div class="animals">
    {#each ANIMALS as animal (animal)}
      <button
        type="button"
        class="animal"
        class:on={chosen === animal}
        aria-pressed={chosen === animal}
        onclick={() => (chosen = animal)}
      >
        <Mascot species={animal} mood={chosen === animal ? 'happy' : 'idle'} size={180} />
        <span>{t(`animals.${animal}`)}</span>
      </button>
    {/each}
  </div>
  <form onsubmit={adopt}>
    <span class="name">
      <TextField
        label={t('adopt.name')}
        hideLabel
        placeholder={t('adopt.placeholder')}
        maxlength={20}
        bind:value={name}
      />
    </span>
    <Button type="submit" variant="primary" disabled={!name.trim()}>{t('adopt.confirm')}</Button>
  </form>
</div>

<style>
  .adopt {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 18px;
  }

  .animals {
    display: grid;
    grid-template-columns: repeat(4, 200px);
    gap: 12px;
  }

  .animal {
    height: 248px;
    padding: 10px;
    border-radius: 16px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    font-size: 15px;
    font-weight: 600;
    background: var(--surface);
    border: 1px solid var(--border);
    transition:
      background-color var(--motion-fast),
      transform var(--motion-fast);
  }

  .animal:hover {
    background: var(--surface-hover);
  }

  .animal:active {
    transform: scale(0.985);
  }

  .animal.on {
    background: var(--surface-selected);
    border-color: var(--accent);
  }

  form {
    width: 400px;
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .name {
    flex: 1;
    min-width: 0;
  }

  form :global(.button) {
    height: 44px;
    padding: 0 22px;
    font-size: 14px;
  }
</style>
