<script lang="ts">
  import { Button, Modal } from '@arthex/sdk';
  import { t } from './i18n';
  import { remote } from './remote.svelte';

  const trusting = $derived(remote.trusting);
  const shown = $derived(trusting ? remote.statusOf(trusting.id) : null);
</script>

{#if trusting && shown?.state === 'trust'}
  <Modal title={t('trust.title')} onclose={() => remote.refuse(trusting)}>
    <p>{t('trust.text', { host: trusting.host })}</p>
    <div class="print">
      <span class="kind">{t('trust.kind', { algorithm: shown.algorithm })}</span>
      <span class="value">{shown.fingerprint}</span>
    </div>
    {#snippet actions()}
      <Button onclick={() => remote.refuse(trusting)}>{t('cancel')}</Button>
      <Button variant="primary" onclick={() => remote.trust(trusting)}>{t('trust.action')}</Button>
    {/snippet}
  </Modal>
{:else if remote.removing}
  {@const removing = remote.removing}
  <Modal title={t('removeTitle', { name: removing.name })} onclose={() => (remote.removing = null)}>
    <p>{t('removeText')}</p>
    {#snippet actions()}
      <Button onclick={() => (remote.removing = null)}>{t('cancel')}</Button>
      <Button variant="quiet-danger" onclick={() => remote.remove(removing)}>{t('removeAction')}</Button>
    {/snippet}
  </Modal>
{/if}

<style>
  p {
    margin: 0;
    font-size: 14px;
    line-height: 1.5;
    color: var(--text);
  }

  .print {
    padding: 12px 14px;
    border-radius: 12px;
    background: var(--surface-sunken);
    border: 1px solid var(--border-strong);
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .kind {
    font-size: 12.5px;
    color: var(--text-subtle);
  }

  .value {
    font-family: var(--font-mono);
    font-size: 13px;
    color: var(--text);
    overflow-wrap: anywhere;
  }
</style>
