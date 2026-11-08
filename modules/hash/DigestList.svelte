<script lang="ts">
  import { Skeleton, toast } from '@arthex/sdk';
  import { algorithm } from './algorithms';
  import { matches, shown, type Format } from './formats';
  import type { Entry } from './hash.svelte';
  import { t } from './i18n';
  import { CHECK } from './identity';

  interface Props {
    entries: readonly Entry[];
    format: Format;
    loading?: boolean;
    /** "HMAC " before every name, when the digests are keyed. */
    prefix?: string;
    compare: string;
  }

  let { entries, format, loading = false, prefix = '', compare }: Props = $props();

  const rows = $derived(
    entries.map((entry, index) => {
      const group = algorithm(entry.id)?.group ?? 'classic';
      const previous = entries[index - 1];
      return {
        ...entry,
        name: prefix + (algorithm(entry.id)?.name ?? entry.id),
        value: entry.hex === null ? '—' : shown(entry.hex, format),
        matched: entry.hex !== null && matches(entry.hex, compare),
        heading: previous === undefined || algorithm(previous.id)?.group !== group ? group : null,
      };
    }),
  );

  async function copy(name: string, value: string): Promise<void> {
    await navigator.clipboard.writeText(value);
    toast(t('copied', { name }));
  }
</script>

<div class="digests">
  <h3>{t('hashes')}</h3>
  <section class="list scroll">
    {#each rows as row (row.id)}
      {#if row.heading}
        <h4>{t(`groups.${row.heading}`)}</h4>
      {/if}
      <button
        type="button"
        class="row"
        class:matched={row.matched}
        disabled={row.hex === null}
        aria-label="{row.name} : {row.value}"
        onclick={() => copy(row.name, row.value)}
      >
        <span class="name">{row.name}</span>
        {#if loading}
          <span class="value"><Skeleton width="100%" height="14px" /></span>
        {:else}
          <span class="value" class:empty={row.hex === null}>{row.value}</span>
        {/if}
        {#if row.matched}
          <svg class="check" aria-hidden="true" width="16" height="16" viewBox="0 0 24 24"
            ><path d={CHECK} /></svg
          >
        {/if}
      </button>
    {/each}
  </section>
</div>

<style>
  .digests {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  h3 {
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-subtle);
  }

  .list {
    flex: 1;
    min-height: 0;
    overflow-x: hidden;
    padding: 4px 10px;
    border-radius: var(--radius-l);
    background: var(--surface);
    border: 1px solid var(--border);
  }

  h4 {
    padding: 12px 8px 4px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-subtle);
  }

  .row {
    width: 100%;
    padding: 9px 8px;
    border-radius: var(--radius-s);
    display: flex;
    align-items: flex-start;
    gap: 14px;
    text-align: left;
    transition: background-color var(--motion-fast);
  }

  .row:not(:disabled):hover {
    background: var(--surface-hover);
  }

  .row:disabled {
    cursor: default;
  }

  .row.matched {
    background: color-mix(in srgb, var(--success) 14%, transparent);
  }

  .name {
    width: 96px;
    flex: none;
    padding-top: 1px;
    font-size: 12.5px;
    color: var(--text-muted);
  }

  .value {
    flex: 1;
    min-width: 0;
    font-family: var(--font-mono);
    font-size: 12.5px;
    line-height: 1.5;
    color: var(--text);
    overflow-wrap: anywhere;
  }

  .value.empty {
    color: var(--text-subtle);
  }

  .check {
    flex: none;
    margin-top: 1px;
    fill: none;
    stroke: var(--success);
    stroke-width: 2.4;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
</style>
