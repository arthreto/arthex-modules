<script lang="ts">
  import { calendar } from './calendar.svelte';
  import CalendarBar from './CalendarBar.svelte';
  import { addDays, addMonths, monthOf, weeksOf, yearOf } from './dates';
  import { monthName, shortDay, weekdayNames } from './format';
  import { dayOf } from './items';
  import { placeWeek } from './layout';

  let { year }: { year: boolean } = $props();

  const LINES = 2;

  const first = $derived(year ? yearOf(calendar.focus) : monthOf(calendar.focus));
  const size = $derived(year ? ('line' as const) : ('small' as const));
  const firstLine = $derived(year ? 14 : 25);
  const lineHeight = $derived(year ? 3 : 17);
  const names = $derived(weekdayNames());
  const eventDays = $derived(
    new Set(calendar.items.filter((item) => item.kind === 'event').map((item) => dayOf(item.start))),
  );

  const months = $derived(
    Array.from({ length: year ? 12 : 3 }, (_, index) => {
      const start = addMonths(first, index);
      const month = start.slice(0, 7);
      return {
        start,
        month,
        title: monthName(start, !year),
        weeks: weeksOf(start).map((monday) => {
          const days = Array.from({ length: 7 }, (_, column) => addDays(monday, column));
          const own = days.flatMap((day, column) => (day.startsWith(month) ? [column] : []));
          const { placed } = placeWeek(calendar.items, monday, calendar.today, {
            lanes: LINES,
            events: false,
            firstColumn: own[0] ?? 7,
            lastColumn: own.at(-1) ?? -1,
          });
          return { monday, days, placed };
        }),
      };
    }),
  );
</script>

<div class="months" class:year>
  {#each months as month (month.start)}
    <div class="month">
      <button type="button" class="title" onclick={() => calendar.openMonth(month.start)}
        >{month.title}</button
      >
      {#if !year}
        <div class="names">
          {#each names as name (name)}
            <span>{name}</span>
          {/each}
        </div>
      {/if}
      {#each month.weeks as week (week.monday)}
        <div class="week">
          {#each week.days as day (day)}
            {@const own = day.startsWith(month.month)}
            <button
              type="button"
              class="day"
              aria-label={shortDay(day)}
              onclick={() => calendar.openMonth(day)}
            >
              <span class="number" class:today={own && day === calendar.today} class:other={!own}
                >{Number(day.slice(8))}</span
              >
              {#if !year && own && eventDays.has(day)}
                <span class="dot"></span>
              {/if}
            </button>
          {/each}
          {#each week.placed as part (part.item.id)}
            <CalendarBar {part} {size} top={firstLine + part.lane * lineHeight} />
          {/each}
        </div>
      {/each}
    </div>
  {/each}
</div>

<style>
  .months {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    grid-auto-rows: minmax(0, 1fr);
    gap: 10px;
  }

  .year {
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 8px;
  }

  .month {
    min-height: 0;
    padding: 10px 10px 8px;
    border-radius: 12px;
    background: var(--surface);
    border: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .year .month {
    padding: 8px 10px 6px;
  }

  .title {
    flex: none;
    font-size: 14px;
    font-weight: 600;
    text-align: left;
  }

  .year .title {
    font-size: 12.5px;
  }

  .title:hover {
    color: var(--accent);
  }

  .names {
    flex: none;
    display: grid;
    grid-template-columns: repeat(7, minmax(0, 1fr));
    font-size: 10.5px;
    color: var(--text-subtle);
  }

  .names span {
    padding-left: 4px;
  }

  .week {
    position: relative;
    flex: 1 1 0;
    min-height: 0;
    display: grid;
    grid-template-columns: repeat(7, minmax(0, 1fr));
  }

  .day {
    min-width: 0;
    display: flex;
    justify-content: flex-start;
    align-items: flex-start;
    gap: 3px;
  }

  .year .day {
    justify-content: center;
  }

  .number {
    width: 20px;
    height: 20px;
    flex: none;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 12px;
    font-weight: 500;
  }

  .year .number {
    width: 16px;
    height: 16px;
    font-size: 10.5px;
  }

  .day:hover .number {
    background: var(--surface-hover);
  }

  .other {
    color: var(--text-subtle);
    opacity: 0.6;
  }

  .today,
  .day:hover .today {
    background: var(--accent);
    color: var(--on-accent);
    font-weight: 700;
  }

  .dot {
    width: 5px;
    height: 5px;
    margin-top: 7px;
    border-radius: 50%;
    background: var(--text-muted);
  }
</style>
