<script lang="ts">
  import { LETTERS } from './connection';
  import { t } from './i18n';

  interface Props {
    value: string;
    /** Letters of the other connections, not offered. */
    used: readonly string[];
    disabled: boolean;
    onchange: (letter: string) => void;
  }

  let { value, used, disabled, onchange }: Props = $props();
  const letters = $derived(LETTERS.filter((letter) => letter === value || !used.includes(letter)));
</script>

<label class="field">
  <span>{t('letter')}</span>
  <select {value} {disabled} onchange={(event) => onchange(event.currentTarget.value)}>
    <option value="">{t('noLetter')}</option>
    {#each letters as letter (letter)}
      <option value={letter}>{letter}:</option>
    {/each}
  </select>
</label>

<style>
  .field {
    display: flex;
    flex-direction: column;
    gap: 5px;
    min-width: 0;
    font-size: 12.5px;
    color: var(--text-muted);
  }

  select {
    height: 36px;
    padding: 0 12px;
    border-radius: 10px;
    background: var(--surface-sunken);
    border: 1px solid var(--border-strong);
    font: inherit;
    font-size: 13.5px;
    color: var(--text);
    outline: none;
    transition: border-color var(--motion-fast);
  }

  select:focus {
    border-color: var(--accent);
  }

  select:disabled {
    opacity: 0.55;
  }

  option {
    background: var(--surface-dialog);
    color: var(--text);
  }
</style>
