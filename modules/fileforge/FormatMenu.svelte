<script lang="ts">
  import { Button, Menu } from '@arthex/sdk';
  import { formatLabel, groupsOf, type Item } from './formats';
  import { forge } from './forge.svelte';
  import { t } from './i18n';

  /** Keeps a tall menu on the screen: `Menu` only allows for a short one. */
  const MENU_HEIGHT = 340;

  let { item, x, y, onclose }: { item: Item; x: number; y: number; onclose: () => void } = $props();

  function pick(format: string) {
    void forge.setTarget(item.id, format);
    onclose();
  }

  function applyToAll() {
    void forge.setKindTarget(item.kind, item.target);
    onclose();
  }
</script>

<Menu {x} y={Math.min(y, window.innerHeight - MENU_HEIGHT)} label={t('outputFormat')} {onclose}>
  <div class="content">
    {#each groupsOf(item.kind) as { group, formats } (group)}
      <section>
        <h4>{t(`groups.${group}`)}</h4>
        <div class="chips">
          {#each formats as format (format)}
            <button
              class="chip"
              class:on={format === item.target}
              role="menuitem"
              aria-current={format === item.target ? 'true' : undefined}
              onclick={() => pick(format)}
            >
              {formatLabel(format)}
            </button>
          {/each}
        </div>
      </section>
    {/each}
    <Button size="small" onclick={applyToAll}>{t(`applyTo.${item.kind}`)}</Button>
  </div>
</Menu>

<style>
  .content {
    padding: 6px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  section {
    display: flex;
    flex-direction: column;
    gap: 7px;
  }

  h4 {
    margin: 0;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--text-subtle);
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 5px;
  }

  .chip {
    height: 26px;
    padding: 0 9px;
    border-radius: var(--radius-pill);
    background: var(--surface-raised);
    border: 1px solid var(--border-strong);
    font-size: 12px;
    color: var(--text-muted);
    transition: background-color var(--motion-fast);
  }

  .chip:hover {
    background: var(--surface-hover);
  }

  .chip.on {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--on-accent);
  }

  .content > :global(.button) {
    width: 100%;
    height: auto;
    min-height: 28px;
    padding: 5px 8px;
    white-space: normal;
  }
</style>
