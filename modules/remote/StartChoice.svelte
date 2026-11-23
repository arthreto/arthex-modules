<script lang="ts">
  import { Icon } from '@arthex/sdk';
  import { PROTOCOLS, type Protocol } from './connection';
  import { t } from './i18n';
  import { remote } from './remote.svelte';

  const ICONS: Record<Protocol, string> = {
    sftp: 'M4 4h16v16H4zM8 9l3 3-3 3M13 15h3',
    ftp: 'M4 7a2 2 0 0 1 2-2h4l2 2h6a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2z',
    ftps: 'M6 11h12v9H6zM9 11V8a3 3 0 0 1 6 0v3',
  };
</script>

<section class="choice">
  <h3>{t('nothingSelected')}</h3>
  <div class="starts">
    {#each PROTOCOLS as protocol (protocol)}
      <button class="start" onclick={() => remote.create(protocol)}>
        <span class="tile"><Icon path={ICONS[protocol]} size={20} /></span>
        <span class="text">
          <span class="label">{protocol.toUpperCase()}</span>
          <span class="sub">{t(`starts.${protocol}`)}</span>
        </span>
      </button>
    {/each}
  </div>
</section>

<style>
  .choice {
    flex: 1;
    min-width: 0;
    padding: 20px;
    border-radius: 16px;
    background: var(--surface);
    border: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 20px;
  }

  h3 {
    margin: 0;
    font-size: 18px;
    font-weight: 600;
  }

  .starts {
    width: 100%;
    max-width: 520px;
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 10px;
  }

  .start {
    padding: 14px;
    border-radius: 14px;
    background: var(--surface);
    border: 1px solid var(--border);
    display: flex;
    align-items: center;
    gap: 12px;
    text-align: left;
    transition: background-color var(--motion-fast);
  }

  .start:hover {
    background: var(--surface-hover);
  }

  .tile {
    width: 40px;
    height: 40px;
    flex: none;
    box-sizing: border-box;
    border-radius: 12px;
    background: var(--surface-raised);
    border: 1px solid var(--border-strong);
    color: var(--tint-blue);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .text {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .label {
    font-size: 14px;
    font-weight: 600;
  }

  .sub {
    font-size: 12px;
    color: var(--text-subtle);
  }
</style>
