<script lang="ts">
  import { calendar } from './calendar.svelte';
  import CalendarBar from './CalendarBar.svelte';
  import { addDays, daysBetween, mondayOf } from './dates';
  import { hourText, itemDescription, timeText, weekdayNames } from './format';
  import { timeOf } from './items';
  import { FIRST_HOUR, LAST_HOUR, placeHours, placeWeek } from './layout';

  const LINE_HEIGHT = 21;
  const LINES = 3;
  const HOURS = LAST_HOUR - FIRST_HOUR;

  const monday = $derived(mondayOf(calendar.focus));
  const days = $derived(Array.from({ length: 7 }, (_, index) => addDays(monday, index)));
  const names = $derived(weekdayNames());
  const bars = $derived(
    placeWeek(
      calendar.items.filter((item) => item.kind === 'range'),
      monday,
      calendar.today,
      { lanes: LINES, events: false },
    ).placed,
  );
  const lines = $derived(Math.max(1, ...bars.map((part) => part.lane + 1)));
  const blocks = $derived(placeHours(calendar.items, monday));
  const hours = Array.from({ length: HOURS }, (_, index) => FIRST_HOUR + index);
  const todayColumn = $derived(daysBetween(monday, calendar.today));
  const showsNow = $derived(todayColumn >= 0 && todayColumn < 7);
  const nowTop = $derived.by(() => {
    const [hour = 0, minute = 0] = timeOf(calendar.now).split(':').map(Number);
    return (Math.min(LAST_HOUR, Math.max(FIRST_HOUR, hour + minute / 60)) - FIRST_HOUR) / HOURS;
  });
</script>

<div class="view">
  <div class="row head">
    <div class="columns">
      {#each days as day, index (day)}
        <button
          type="button"
          class="day"
          class:selected={day === calendar.selected}
          onclick={() => calendar.select(day)}
          ondblclick={() => calendar.add(day)}
        >
          <span class="name">{names[index]}</span>
          <span class="number" class:today={day === calendar.today}>{Number(day.slice(8))}</span>
        </button>
      {/each}
    </div>
  </div>
  <div class="row deadlines">
    <div class="lines" style:height="{lines * LINE_HEIGHT + 2}px">
      {#each bars as part (part.item.id)}
        <CalendarBar {part} size="large" top={2 + part.lane * LINE_HEIGHT} />
      {/each}
    </div>
  </div>
  <div class="hours">
    <div class="labels">
      {#each hours as hour (hour)}
        <span style:top="{((hour - FIRST_HOUR) / HOURS) * 100}%">{hourText(hour)}</span>
      {/each}
    </div>
    <div class="area">
      {#each hours as hour (hour)}
        <span class="rule" style:top="{((hour - FIRST_HOUR) / HOURS) * 100}%"></span>
      {/each}
      {#each blocks as block (block.item.id)}
        {@const width = 100 / 7 / block.sides}
        <button
          type="button"
          class="block"
          class:short={block.short}
          style:--tint="var(--tint-{block.item.tint})"
          style:left="calc({(block.column / 7) * 100 + block.side * width}% + 2px)"
          style:width="calc({width}% - 4px)"
          style:top="{block.top * 100}%"
          style:height="{block.height * 100}%"
          title={itemDescription(block.item)}
          aria-label={itemDescription(block.item)}
          onclick={() => calendar.edit(block.item)}
        >
          <span class="title">{block.item.title}</span>
          <span class="time">{timeText(block.item.start)}</span>
        </button>
      {/each}
      {#if showsNow}
        <span
          class="now"
          style:left="{(todayColumn / 7) * 100}%"
          style:width="{100 / 7}%"
          style:top="{nowTop * 100}%"
        ></span>
      {/if}
    </div>
  </div>
</div>

<style>
  .view {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    border-radius: 14px;
    background: var(--surface);
    border: 1px solid var(--border);
  }

  .row {
    flex: none;
    padding-left: 46px;
  }

  .deadlines {
    border-top: 1px solid var(--border);
  }

  .columns {
    display: grid;
    grid-template-columns: repeat(7, minmax(0, 1fr));
  }

  .day {
    padding: 6px 8px;
    display: flex;
    align-items: center;
    gap: 7px;
  }

  .day:hover {
    background: var(--surface-hover);
  }

  .selected,
  .selected:hover {
    background: var(--surface-selected);
  }

  .name {
    font-size: 12px;
    color: var(--text-subtle);
  }

  .number {
    width: 24px;
    height: 24px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 13px;
    font-weight: 600;
  }

  .today {
    background: var(--accent);
    color: var(--on-accent);
  }

  .lines {
    position: relative;
  }

  .hours {
    position: relative;
    flex: 1;
    min-height: 0;
    display: flex;
    border-top: 1px solid var(--border);
  }

  .labels {
    position: relative;
    width: 46px;
    flex: none;
  }

  .labels span {
    position: absolute;
    right: 8px;
    margin-top: 2px;
    font-size: 11px;
    color: var(--text-subtle);
  }

  .area {
    position: relative;
    flex: 1;
    min-width: 0;
  }

  .rule {
    position: absolute;
    left: 0;
    right: 0;
    border-top: 1px solid var(--border);
  }

  .block {
    position: absolute;
    min-height: 19px;
    box-sizing: border-box;
    padding: 2px 6px;
    overflow: hidden;
    border-radius: 6px;
    background: color-mix(in srgb, var(--tint) 20%, transparent);
    border-left: 3px solid var(--tint);
    display: flex;
    flex-direction: column;
    font-size: 11.5px;
    line-height: 1.25;
    text-align: left;
  }

  .block:hover {
    background: color-mix(in srgb, var(--tint) 30%, transparent);
  }

  .short {
    flex-direction: row;
    align-items: center;
    gap: 6px;
  }

  .title {
    min-width: 0;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .time {
    flex: none;
    color: var(--text-muted);
  }

  .now {
    position: absolute;
    border-top: 2px solid var(--accent);
    pointer-events: none;
  }
</style>
