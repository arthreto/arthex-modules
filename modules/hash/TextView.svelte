<script lang="ts">
  import { Switch, TextField, toast } from '@arthex/sdk';
  import { ALGORITHMS, algorithm } from './algorithms';
  import Comparison from './Comparison.svelte';
  import DigestList from './DigestList.svelte';
  import { matches, type Format } from './formats';
  import { digestsOf, type Digest } from './hash.svelte';
  import { t } from './i18n';

  let { format, text: initial }: { format: Format; text: string } = $props();

  // svelte-ignore state_referenced_locally
  let text = $state(initial);
  let hmac = $state(false);
  let key = $state('');
  let compare = $state('');
  let digests = $state<Digest[]>([]);
  let asked = 0;

  $effect(() => {
    const run = ++asked;
    digestsOf(text, hmac ? key : undefined).then(
      (found) => {
        if (run === asked) digests = found;
      },
      () => toast(t('failed')),
    );
  });

  const entries = $derived(
    ALGORITHMS.filter((a) => !hmac || a.hmac).map((a) => ({
      id: a.id,
      hex: digests.find((digest) => digest.id === a.id)?.hex ?? null,
    })),
  );
  const prefix = $derived(hmac ? 'HMAC ' : '');
  const matched = $derived(entries.find((entry) => entry.hex !== null && matches(entry.hex, compare)));

  function focusOnMount(node: HTMLTextAreaElement) {
    node.focus();
  }
</script>

<div class="view">
  <div class="source">
    <label class="text">
      <span>{t('text')}</span>
      <textarea
        class="scroll"
        bind:value={text}
        use:focusOnMount
        placeholder={t('textPlaceholder')}
        spellcheck="false"
      ></textarea>
    </label>
    <div class="hmac">
      <span>{t('hmac')}</span>
      <Switch label={t('hmac')} checked={hmac} onchange={(checked) => (hmac = checked)} />
    </div>
    {#if hmac}
      <TextField
        label={t('hmacKey')}
        hideLabel
        bind:value={key}
        placeholder={t('hmacPlaceholder')}
        autocomplete="off"
      />
    {/if}
    <Comparison
      bind:value={compare}
      ready
      matched={matched ? prefix + (algorithm(matched.id)?.name ?? '') : null}
    />
  </div>
  <DigestList {entries} {format} {prefix} {compare} />
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

  .text {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 7px;
    font-size: 13px;
    color: var(--text-muted);
  }

  textarea {
    flex: 1;
    min-height: 0;
    padding: 12px 14px;
    border-radius: 10px;
    background: var(--surface-sunken);
    border: 1px solid var(--border-strong);
    font: inherit;
    font-size: 14px;
    line-height: 1.5;
    color: var(--text);
    resize: none;
    outline: none;
    transition: border-color var(--motion-fast);
  }

  textarea:focus {
    border-color: var(--accent);
  }

  textarea::placeholder {
    color: var(--text-subtle);
  }

  .hmac {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    font-size: 13px;
    color: var(--text-muted);
  }
</style>
