<script lang="ts">
  import { Button, Icon } from '@arthex/sdk';
  import type { Connection } from './connection';
  import { t } from './i18n';
  import { placeOf, remote } from './remote.svelte';

  const PLUG = 'M9 2v6M15 2v6M6 8h12v3a6 6 0 0 1-12 0zM12 17v5';
  const UNPLUG = 'M9 2v6M15 2v6M6 8h12v3a6 6 0 0 1-12 0zM12 17v5M3 3l18 18';

  function subtitle(connection: Connection): string {
    const status = remote.statusOf(connection.id);
    if (status.state === 'busy') return t('status.busy');
    if (status.state === 'error') return t('status.failed');
    const place = placeOf(connection);
    return place ? `${connection.protocol.toUpperCase()} · ${place}` : connection.protocol.toUpperCase();
  }
</script>

<div class="list scroll">
  <Button onclick={() => remote.create('sftp')}>
    <Icon name="plus" size={14} stroke={2.2} />{t('newConnection')}
  </Button>
  {#each remote.connections as connection (connection.id)}
    {@const state = remote.statusOf(connection.id).state}
    {@const on = state === 'on'}
    <div class="entry" class:on={connection.id === remote.selected}>
      <button
        class="pick"
        aria-current={connection.id === remote.selected ? 'true' : undefined}
        onclick={() => remote.select(connection.id)}
      >
        <span class="name">{connection.name}</span>
        <span class="sub">
          {#if on || state === 'error'}<span class="dot" class:error={state === 'error'} aria-hidden="true"></span>{/if}
          {subtitle(connection)}
        </span>
      </button>
      <button
        class="toggle"
        aria-label={t(on ? 'disconnectNamed' : 'connectNamed', { name: connection.name })}
        title={t(on ? 'disconnectNamed' : 'connectNamed', { name: connection.name })}
        disabled={state === 'busy' || (!on && !remote.canConnect(connection))}
        onclick={() => remote.toggle(connection)}
      >
        <Icon path={on ? UNPLUG : PLUG} size={15} />
      </button>
    </div>
  {/each}
</div>

<style>
  .list {
    width: 250px;
    flex: none;
    padding-right: 6px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .list > :global(.button) {
    width: 100%;
    margin-bottom: 8px;
  }

  .entry {
    width: 100%;
    padding-right: 6px;
    box-sizing: border-box;
    border-radius: 10px;
    display: flex;
    align-items: center;
    gap: 4px;
    transition: background-color var(--motion-fast);
  }

  .entry:hover {
    background: var(--surface-hover);
  }

  .entry.on {
    background: var(--surface-selected);
  }

  .pick {
    flex: 1;
    min-width: 0;
    padding: 10px 6px 10px 12px;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 3px;
    text-align: left;
  }

  .name {
    max-width: 100%;
    overflow: hidden;
    font-size: 14px;
    font-weight: 500;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .sub {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12.5px;
    color: var(--text-subtle);
  }

  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--tint-green);
  }

  .dot.error {
    background: var(--tint-red);
  }

  .toggle {
    width: 32px;
    height: 32px;
    flex: none;
    border-radius: 8px;
    color: var(--text-muted);
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background-color var(--motion-fast);
  }

  .toggle:hover:enabled {
    background: var(--surface-hover);
    color: var(--text);
  }

  .toggle:disabled {
    opacity: 0.4;
  }
</style>
