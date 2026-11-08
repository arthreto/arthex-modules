<script lang="ts">
  import { Button, toast, ToggleChip } from '@arthex/sdk';
  import { keyText, type KeyFormat } from './formats';
  import { t } from './i18n';

  const LENGTHS = [128, 256, 512] as const;
  const FORMATS: { value: KeyFormat; label: string }[] = [
    { value: 'hex', label: 'Hex' },
    { value: 'base64', label: 'Base64' },
    { value: 'base64url', label: 'Base64 URL' },
    { value: 'uuid', label: 'UUID' },
  ];

  /** Enough for the longest key; the system's generator, fit for secrets. */
  const fresh = () => crypto.getRandomValues(new Uint8Array(64));

  let bits = $state<number>(256);
  let format = $state<KeyFormat>('hex');
  let bytes = $state(fresh());

  const key = $derived(keyText(bytes, bits, format));

  async function copy(): Promise<void> {
    await navigator.clipboard.writeText(key);
    toast(t('keyCopied'));
  }
</script>

<div class="view">
  <div class="field">
    <span>{t('length')}</span>
    <div class="chips" role="group" aria-label={t('length')}>
      {#each LENGTHS as length (length)}
        <ToggleChip
          label={t('bits', { count: length })}
          pressed={bits === length && format !== 'uuid'}
          disabled={format === 'uuid'}
          onclick={() => (bits = length)}
        />
      {/each}
    </div>
  </div>
  <div class="field">
    <span>{t('keyFormat')}</span>
    <div class="chips" role="group" aria-label={t('keyFormat')}>
      {#each FORMATS as option (option.value)}
        <ToggleChip
          label={option.label}
          pressed={format === option.value}
          onclick={() => (format = option.value)}
        />
      {/each}
    </div>
  </div>
  <div class="key" aria-live="polite">{key}</div>
  <div class="actions">
    <Button size="large" onclick={() => (bytes = fresh())}>{t('newKey')}</Button>
    <Button variant="primary" size="large" onclick={copy}>{t('copy')}</Button>
  </div>
</div>

<style>
  .view {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 18px;
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

  .key {
    flex: 1;
    min-height: 0;
    padding: 18px 20px;
    border-radius: var(--radius-l);
    background: var(--surface-sunken);
    border: 1px solid var(--border-strong);
    display: flex;
    align-items: center;
    font-family: var(--font-mono);
    font-size: 17px;
    line-height: 1.6;
    overflow-wrap: anywhere;
  }

  .actions {
    flex: none;
    display: flex;
    justify-content: flex-end;
    gap: 10px;
  }
</style>
