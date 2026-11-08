<script lang="ts">
  import { Button, Skeleton, TextField, toast, ToggleChip } from '@arthex/sdk';
  import { PASSWORD_KINDS, PASSWORD_NAMES, type PasswordKind } from './algorithms';
  import { checkPassword, hashPassword } from './hash.svelte';
  import { t } from './i18n';
  import { CHECK } from './identity';

  const CHECK_AFTER_MS = 400;

  let { stored: initial }: { stored: string } = $props();

  let password = $state('');
  let kind = $state<PasswordKind>('argon2id');
  let busy = $state(false);
  let result = $state<{ kind: PasswordKind; hash: string } | null>(null);
  // svelte-ignore state_referenced_locally
  let stored = $state(initial);
  let verdict = $state<'checking' | 'matches' | 'differs' | 'unknown' | null>(null);
  let checks = 0;

  const shownKind = $derived(result?.kind ?? kind);

  $effect(() => {
    const run = ++checks;
    if (password === '' || stored.trim() === '') {
      verdict = null;
      return;
    }
    verdict = 'checking';
    const timer = setTimeout(() => {
      checkPassword(password, stored).then(
        (ok) => {
          if (run === checks) verdict = ok ? 'matches' : 'differs';
        },
        () => {
          if (run === checks) verdict = 'unknown';
        },
      );
    }, CHECK_AFTER_MS);
    return () => clearTimeout(timer);
  });

  async function hash(): Promise<void> {
    busy = true;
    try {
      result = { kind, hash: await hashPassword(kind, password) };
    } catch {
      toast(t('failed'));
    } finally {
      busy = false;
    }
  }

  async function copy(): Promise<void> {
    if (!result) return;
    await navigator.clipboard.writeText(result.hash);
    toast(t('hashCopied'));
  }
</script>

<div class="view">
  <div class="source">
    <TextField label={t('password')} type="password" bind:value={password} autocomplete="off" autofocus />
    <div class="field">
      <span>{t('algorithm')}</span>
      <div class="chips" role="group" aria-label={t('algorithm')}>
        {#each PASSWORD_KINDS as option (option)}
          <ToggleChip
            label={PASSWORD_NAMES[option]}
            pressed={kind === option}
            onclick={() => (kind = option)}
          />
        {/each}
      </div>
    </div>
    <Button variant="primary" size="large" disabled={password === '' || busy} onclick={hash}>
      {t('hashPassword')}
    </Button>
    <div class="spacer"></div>
    <div class="stored">
      <TextField
        label={t('verify')}
        bind:value={stored}
        placeholder={t('verifyPlaceholder')}
        spellcheck="false"
        autocomplete="off"
      />
    </div>
    {#if verdict}
      <span class="verdict {verdict}" role="status">
        {#if verdict === 'matches'}
          <svg aria-hidden="true" width="15" height="15" viewBox="0 0 24 24"><path d={CHECK} /></svg>
          {t('passwordMatches')}
        {:else if verdict === 'differs'}
          {t('passwordDiffers')}
        {:else if verdict === 'unknown'}
          {t('unknownHash')}
        {:else}
          {t('verifying')}
        {/if}
      </span>
    {/if}
  </div>
  <div class="side">
    <h3>{t('result')}</h3>
    <div class="box">
      {#if busy}
        <div class="loading" aria-busy="true">
          <Skeleton width="100%" height="16px" />
          <Skeleton width="70%" height="16px" />
        </div>
      {:else}
        <span class="hash" class:empty={!result} aria-live="polite">{result?.hash ?? '—'}</span>
      {/if}
      <div class="foot">
        <span class="params">{PASSWORD_NAMES[shownKind]} · {t(`params.${shownKind}`)}</span>
        <Button disabled={!result || busy} onclick={copy}>{t('copy')}</Button>
      </div>
    </div>
  </div>
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

  .stored :global(input) {
    font-family: var(--font-mono);
    font-size: 13px;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 7px;
    font-size: 13px;
    color: var(--text-muted);
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .spacer {
    flex: 1;
  }

  .verdict {
    margin-top: -6px;
    display: flex;
    align-items: center;
    gap: 7px;
    font-size: 13px;
    color: var(--danger-text);
  }

  .verdict.matches {
    color: var(--success);
  }

  .verdict.checking {
    color: var(--text-muted);
  }

  svg {
    flex: none;
    fill: none;
    stroke: currentColor;
    stroke-width: 2.4;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .side {
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

  .box {
    flex: 1;
    min-height: 0;
    padding: 18px 20px;
    border-radius: var(--radius-l);
    background: var(--surface);
    border: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .loading {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .hash {
    font-family: var(--font-mono);
    font-size: 16px;
    line-height: 1.6;
    overflow-wrap: anywhere;
  }

  .hash.empty {
    color: var(--text-subtle);
  }

  .foot {
    margin-top: auto;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .params {
    font-size: 12.5px;
    color: var(--text-muted);
  }
</style>
