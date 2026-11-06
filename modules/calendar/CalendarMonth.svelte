<script lang="ts">
  import { calendar } from './calendar.svelte';
  import CalendarBar from './CalendarBar.svelte';
  import CalendarDayPanel from './CalendarDayPanel.svelte';
  import { addDays, weeksOf, type Day } from './dates';
  import { shortDay, weekdayNames } from './format';
  import { placeWeek } from './layout';

  const FIRST_LINE = 30;
  const LINE_HEIGHT = 20;
  const LINES = 2;

  const month = $derived(calendar.focus.slice(0, 7));
  const weeks = $derived(
    weeksOf(calendar.focus).map((monday) => ({
      monday,
      days: Array.from({ length: 7 }, (_, index) => addDays(monday, index)),
      ...placeWeek(calendar.items, monday, calendar.today, { lanes: LINES, events: true }),
    })),
  );
  const names = $derived(weekdayNames());

  const isToday = (day: Day) => day === calendar.today;
</script>

<div class="view">
  <div class="grid">
    <div class="names">
      {#each names as name (name)}
        <span>{name}</span>
      {/each}
    </div>
    {#each weeks as week (week.monday)}
      <div class="week">
        {#each week.days as day, column (day)}
          {@const hidden = week.hidden[column] ?? 0}
          <button
            type="button"
            class="day"
            class:selected={day === calendar.selected}
            aria-label={shortDay(day)}
            onclick={() => calendar.select(day)}
            ondblclick={() => calendar.add(day)}
          >
            <span class="number" class:today={isToday(day)} class:other={!day.startsWith(month)}
              >{Number(day.slice(8))}</span
            >
            {#if hidden > 0}
              <span class="more">+{hidden}</span>
            {/if}
          </button>
        {/each}
        {#each week.placed as part (part.item.id)}
          <CalendarBar {part} size="large" top={FIRST_LINE + part.lane * LINE_HEIGHT} />
        {/each}
      </div>
    {/each}
  </div>
  <CalendarDayPanel />
</div>

<style>
  .view {
    flex: 1;
    min-height: 0;
    display: flex;
    gap: 12px;
  }

  .grid {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    border-radius: 14px;
    background: var(--surface);
    border: 1px solid var(--border);
  }

  .names {
    flex: none;
    height: 28px;
    display: grid;
    grid-template-columns: repeat(7, minmax(0, 1fr));
    align-items: center;
    font-size: 11.5px;
    color: var(--text-subtle);
  }

  .names span {
    padding-left: 10px;
  }

  .week {
    position: relative;
    flex: 1 1 0;
    min-height: 0;
    display: grid;
    grid-template-columns: repeat(7, minmax(0, 1fr));
    border-top: 1px solid var(--border);
  }

  .day {
    min-width: 0;
    padding: 5px 6px;
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    text-align: left;
  }

  .day + .day {
    border-left: 1px solid var(--border);
  }

  .day:hover {
    background: var(--surface-hover);
  }

  .selected,
  .selected:hover {
    background: var(--surface-selected);
  }

  .number {
    width: 22px;
    height: 22px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 12px;
    font-weight: 500;
  }

  .other {
    color: var(--text-subtle);
    opacity: 0.6;
  }

  .today {
    background: var(--accent);
    color: var(--on-accent);
    font-weight: 700;
  }

  .more {
    font-size: 11px;
    line-height: 22px;
    color: var(--text-subtle);
  }
</style>
