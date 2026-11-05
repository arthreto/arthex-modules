<script lang="ts">
  import { Button, Icon } from '@arthex/sdk';
  import { t } from './i18n';
  import { presence } from './presence.svelte';
  import { canTurnOn } from './template';

  const PLAY = 'M8 5v14l11-7z';
  const STOP = 'M6 6h12v12H6z';
</script>

<div class="list scroll">
  <Button onclick={() => presence.create('playing')}>
    <Icon name="plus" size={14} stroke={2.2} />{t('newTemplate')}
  </Button>
  {#each presence.templates as template (template.id)}
    {@const active = presence.active === template.id}
    <div class="entry" class:on={template.id === presence.selected}>
      <button
        class="pick"
        aria-current={template.id === presence.selected ? 'true' : undefined}
        onclick={() => presence.select(template.id)}
      >
        <span class="name">{template.name}</span>
        <span class="kind">
          {#if active}<span class="dot" aria-hidden="true"></span>{/if}
          {active ? t('isActive') : t(`kinds.${template.kind}`)}
        </span>
      </button>
      <button
        class="toggle"
        aria-label={t(active ? 'turnOffNamed' : 'turnOnNamed', { name: template.name })}
        title={t(active ? 'turnOffNamed' : 'turnOnNamed', { name: template.name })}
        disabled={!active && !canTurnOn(template)}
        onclick={() => presence.toggle(template)}
      >
        <Icon path={active ? STOP : PLAY} size={15} />
      </button>
    </div>
  {/each}
</div>

<style>
  .list {
    width: 240px;
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

  .kind {
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
