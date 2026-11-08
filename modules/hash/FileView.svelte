<script lang="ts">
  import { Button, FileIcon, formatBytes, formatPercent, toast } from '@arthex/sdk';
  import { onMount } from 'svelte';
  import { ALGORITHMS, algorithm } from './algorithms';
  import Comparison from './Comparison.svelte';
  import DigestList from './DigestList.svelte';
  import { matches, type Format } from './formats';
  import { fileHash } from './hash.svelte';
  import { t } from './i18n';

  let { format, compare: initial }: { format: Format; compare: string } = $props();

  // svelte-ignore state_referenced_locally
  let compare = $state(initial);
  const file = $derived(fileHash.state);
  const busy = $derived(file !== null && file.digests === null && !file.failed);

  const entries = $derived(
    ALGORITHMS.map((a) => ({
      id: a.id,
      hex: file?.digests?.find((digest) => digest.id === a.id)?.hex ?? null,
    })),
  );
  const matched = $derived(entries.find((entry) => entry.hex !== null && matches(entry.hex, compare)));
  const progress = $derived(
    file
      ? t('progress', {
          done: formatBytes(file.done),
          size: formatBytes(file.size),
          percent: formatPercent(file.size === 0 ? 1 : file.done / file.size, 0),
        })
      : '',
  );

  onMount(() => {
    fileHash.load().catch(() => toast(t('failed')));
  });

  function pick(): void {
    fileHash.pick().catch(() => toast(t('fileFailed')));
  }
</script>

<div class="view">
  <div class="source">
    {#if file === null}
      <div class="empty">
        <FileIcon folder={false} size={34} />
        <Button variant="primary" onclick={pick}>{t('chooseFile')}</Button>
      </div>
    {:else}
      <div class="card">
        <div class="file">
          <FileIcon folder={false} size={30} />
          <span class="names">
            <span class="name">{file.name}</span>
            <span class="size">{formatBytes(file.size)}</span>
          </span>
        </div>
        {#if busy}
          <div class="busy">
            <span class="progress">
              <span
                class="bar"
                role="progressbar"
                aria-label={t('computing')}
                aria-valuenow={Math.round((file.done / Math.max(file.size, 1)) * 100)}
                aria-valuemin="0"
                aria-valuemax="100"
                ><span class="fill" style:width="{(file.done / Math.max(file.size, 1)) * 100}%"></span></span
              >
              <span class="progress-text">{progress}</span>
            </span>
            <Button onclick={() => fileHash.cancel()}>{t('cancel')}</Button>
          </div>
        {:else}
          {#if file.failed}
            <span class="failed" role="alert">{t('fileFailed')}</span>
          {/if}
          <span class="change"><Button onclick={pick}>{t('change')}</Button></span>
        {/if}
      </div>
      <div class="spacer"></div>
    {/if}
    <Comparison
      bind:value={compare}
      ready={file?.digests != null}
      matched={matched ? (algorithm(matched.id)?.name ?? null) : null}
    />
  </div>
  <DigestList {entries} {format} {compare} loading={busy} />
</div>

<style>
  .view {
    flex: 1;
    min-height: 0;
    display: flex;
    gap: 28px;
  }

  .source {
    width: 300px;
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .empty {
    flex: 1;
    min-height: 0;
    border-radius: var(--radius-l);
    background: var(--surface);
    border: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 14px;
  }

  .card {
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 14px;
    border-radius: var(--radius-l);
    background: var(--surface);
    border: 1px solid var(--border);
  }

  .file {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .names {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .name {
    font-size: 14px;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .size {
    font-size: 12px;
    color: var(--text-muted);
  }

  .busy {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .progress {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .bar {
    display: block;
    height: 7px;
    border-radius: 4px;
    background: var(--surface-hover);
    overflow: hidden;
  }

  .fill {
    display: block;
    height: 100%;
    border-radius: 4px;
    background: var(--accent);
    transition: width var(--motion-fast) linear;
  }

  .progress-text {
    font-size: 12px;
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
  }

  .failed {
    font-size: 13px;
    color: var(--danger-text);
  }

  .change {
    align-self: flex-start;
  }

  .spacer {
    flex: 1;
  }
</style>
