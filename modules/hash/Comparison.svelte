<script lang="ts">
  import { TextField } from '@arthex/sdk';
  import { t } from './i18n';
  import { CHECK } from './identity';

  interface Props {
    value: string;
    /** Whether there is something to compare with yet. */
    ready: boolean;
    /** The name of the digest the pasted one matches, if any. */
    matched: string | null;
  }

  let { value = $bindable(), ready, matched }: Props = $props();
</script>

<div class="comparison">
  <TextField
    label={t('compare')}
    bind:value
    placeholder={t('comparePlaceholder')}
    spellcheck="false"
    autocomplete="off"
  />
  {#if ready && value.trim() !== ''}
    <span class="verdict" class:matched={matched !== null} role="status">
      {#if matched !== null}
        <svg aria-hidden="true" width="15" height="15" viewBox="0 0 24 24"><path d={CHECK} /></svg>
        {t('matches', { name: matched })}
      {:else}
        {t('noMatch')}
      {/if}
    </span>
  {/if}
</div>

<style>
  .comparison {
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .comparison :global(input) {
    font-family: var(--font-mono);
    font-size: 13px;
  }

  .verdict {
    display: flex;
    align-items: center;
    gap: 7px;
    font-size: 13px;
    color: var(--danger-text);
  }

  .verdict.matched {
    color: var(--success);
  }

  svg {
    flex: none;
    fill: none;
    stroke: currentColor;
    stroke-width: 2.4;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
</style>
