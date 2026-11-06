<script lang="ts">
  import { calendar } from './calendar.svelte';
  import { longDay, rangeSpan, remainingText, timeText } from './format';
  import { t } from './i18n';
  import { coversDay, progressOf } from './items';

  const items = $derived(
    calendar.items
      .filter((item) => coversDay(item, calendar.selected))
      .sort((a, b) => {
        if (a.kind !== b.kind) return a.kind === 'event' ? -1 : 1;
        return a.start < b.start ? -1 : 1;
      }),
  );
</script>

<aside class="panel scroll" aria-label={t('day.label')}>
  <span class="title">{longDay(calendar.selected)}</span>
  {#if items.length === 0}
    <span class="empty">{t('day.empty')}</span>
  {/if}
  {#each items as item (item.id)}
    <button
      type="button"
      class="row"
      style:--tint="var(--tint-{item.tint})"
      onclick={() => calendar.edit(item)}
    >
      <span class="line">
        <span class="dot"></span>
        <span class="name">{item.title}</span>
        {#if item.kind === 'event'}
          <span class="when">{timeText(item.start)}</span>
        {/if}
      </span>
      {#if item.kind === 'range'}
        <span class="track"
          ><span class="done" style:width="{progressOf(item, calendar.today) * 100}%"></span></span
        >
        <span class="facts">
          <span>{rangeSpan(item)}</span>
          <span class="left">{remainingText(item, calendar.today)}</span>
        </span>
      {/if}
    </button>
  {/each}
</aside>

<style>
  .panel {
    width: 248px;
    flex: none;
    box-sizing: border-box;
    padding: 14px 16px;
    border-radius: 14px;
    background: var(--surface);
    border: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .title {
    margin-bottom: 4px;
    font-size: 14.5px;
    font-weight: 600;
  }

  .empty {
    font-size: 13px;
    color: var(--text-subtle);
  }

  .row {
    width: calc(100% + 12px);
    margin: 0 -6px;
    flex: none;
    padding: 7px 6px;
    border-radius: 9px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    text-align: left;
  }

  .row:hover {
    background: var(--surface-hover);
  }

  .line {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .dot {
    width: 8px;
    height: 8px;
    flex: none;
    border-radius: 50%;
    background: var(--tint);
  }

  .name {
    flex: 1;
    min-width: 0;
    font-size: 13.5px;
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .when {
    flex: none;
    font-size: 12px;
    color: var(--text-subtle);
  }

  .track {
    display: block;
    width: 100%;
    height: 5px;
    border-radius: 3px;
    background: color-mix(in srgb, var(--tint) 18%, transparent);
    overflow: hidden;
  }

  .done {
    display: block;
    height: 100%;
    border-radius: 3px;
    background: var(--tint);
  }

  .facts {
    width: 100%;
    display: flex;
    flex-wrap: wrap;
    justify-content: space-between;
    gap: 2px 8px;
    font-size: 12px;
    color: var(--text-subtle);
  }

  .left {
    white-space: nowrap;
  }
</style>
