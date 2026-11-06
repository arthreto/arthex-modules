<script lang="ts">
  import { CardButton, navigate } from '@arthex/sdk';
  import { onMount } from 'svelte';
  import { calendar } from './calendar.svelte';
  import { remainingText, timeText, weekdayName } from './format';
  import { t } from './i18n';
  import { ID } from './identity';
  import { comingUp, dayOf, progressOf, type Item } from './items';

  const SHOWN_EVENTS = 2;

  const coming = $derived(comingUp(calendar.items, calendar.now, calendar.today, SHOWN_EVENTS));
  const empty = $derived(coming.events.length === 0 && !coming.deadline);

  function whenOf(item: Item): string {
    const day = dayOf(item.start);
    const label = day === calendar.today ? t('card.today') : `${weekdayName(day)} ${Number(day.slice(8))}`;
    return `${label} · ${timeText(item.start)}`;
  }

  onMount(() => {
    void calendar.load();
  });
</script>

<CardButton title={t('card.title')} link={t('name')} onclick={() => navigate(ID)}>
  <span class="rows">
    {#if empty}
      <span class="empty">{t('card.empty')}</span>
    {/if}
    {#each coming.events as item (item.id)}
      <span class="row" style:--tint="var(--tint-{item.tint})">
        <span class="dot"></span>
        <span class="name">{item.title}</span>
        <span class="when">{whenOf(item)}</span>
      </span>
    {/each}
    {#if coming.deadline}
      {@const deadline = coming.deadline}
      <span class="deadline" style:--tint="var(--tint-{deadline.tint})">
        <span class="row">
          <span class="name">{deadline.title}</span>
          <span class="when">{remainingText(deadline, calendar.today)}</span>
        </span>
        <span class="track"
          ><span class="done" style:width="{progressOf(deadline, calendar.today) * 100}%"></span></span
        >
      </span>
    {/if}
  </span>
</CardButton>

<style>
  .rows {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 9px;
    font-size: 13px;
  }

  .empty {
    color: var(--text-subtle);
  }

  .row {
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
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .when {
    flex: none;
    font-size: 12px;
    color: var(--text-subtle);
  }

  .deadline {
    display: flex;
    flex-direction: column;
    gap: 5px;
  }

  .track {
    display: block;
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
</style>
