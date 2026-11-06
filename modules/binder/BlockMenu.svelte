<script lang="ts">
  import { Icon } from '@arthex/sdk';
  import type { BlockGroup, BlockId, BlockItem } from './blocks';
  import { t } from './i18n';

  interface Props {
    items: readonly BlockItem[];
    active: number;
    left: number;
    top: number;
    onpick: (id: BlockId) => void;
  }

  let { items, active, left, top, onpick }: Props = $props();

  const GROUPS: readonly BlockGroup[] = ['text', 'blocks', 'media'];
  const activeId = $derived(items[active]?.id);

  function intoView(button: HTMLElement, on: boolean) {
    if (on) button.scrollIntoView({ block: 'nearest' });
    return {
      update(now: boolean) {
        if (now) button.scrollIntoView({ block: 'nearest' });
      },
    };
  }
</script>

<div class="menu scroll" role="menu" aria-label={t('insertLabel')} style:left="{left}px" style:top="{top}px">
  {#each GROUPS as group (group)}
    {@const inGroup = items.filter((item) => item.group === group)}
    {#if inGroup.length > 0}
      <div class="group">{t(`groups.${group}`)}</div>
      {#each inGroup as item (item.id)}
        <button
          type="button"
          class="item"
          class:on={item.id === activeId}
          role="menuitem"
          use:intoView={item.id === activeId}
          onmousedown={(event) => event.preventDefault()}
          onclick={() => onpick(item.id)}
        >
          <Icon path={item.icon} size={16} />
          {t(`blocks.${item.id}`)}
        </button>
      {/each}
    {/if}
  {/each}
  {#if items.length === 0}
    <div class="none">{t('noBlock')}</div>
  {/if}
</div>

<style>
  .menu {
    position: absolute;
    z-index: 5;
    width: 236px;
    max-height: 340px;
    padding: 6px;
    border-radius: 12px;
    background: var(--surface-dialog);
    border: 1px solid var(--border-strong);
    box-shadow: 0 18px 44px rgba(0, 0, 0, 0.5);
    animation: enter 140ms ease-out;
  }

  @keyframes enter {
    from {
      opacity: 0;
      transform: scale(0.97);
    }
  }

  .group {
    padding: 8px 10px 4px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-subtle);
  }

  .item {
    width: 100%;
    height: 34px;
    padding: 0 10px;
    border-radius: 7px;
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 13px;
    text-align: left;
  }

  .item :global(svg) {
    flex: none;
    color: var(--text-muted);
  }

  .item:hover,
  .item.on {
    background: var(--surface-hover);
  }

  .none {
    padding: 10px;
    font-size: 13px;
    color: var(--text-subtle);
  }
</style>
