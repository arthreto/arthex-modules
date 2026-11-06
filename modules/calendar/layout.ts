import { addDays, daysBetween, type Day } from './dates';
import { dayOf, timeOf, type Item } from './items';

const WEEK = 7;

/** A deadline or an event in the row of one week. */
export interface Placed {
  item: Item;
  /** Columns of the week, from `0` for Monday. */
  first: number;
  last: number;
  lane: number;
  /** Whether the item really starts or ends here, or goes on in the next row. */
  opens: boolean;
  closes: boolean;
  /** How much of this part has gone, from `0` to `1`. */
  elapsed: number;
  over: boolean;
}

interface WeekOptions {
  lanes: number;
  events: boolean;
  /** A small month only draws its own days. */
  firstColumn?: number;
  lastColumn?: number;
}

/**
 * Places what happens during a week on as few lines as possible: the deadlines first, as they
 * are the longest. What does not fit is counted for each day, which says "+1".
 */
export function placeWeek(items: readonly Item[], monday: Day, today: Day, options: WeekOptions) {
  const sunday = addDays(monday, WEEK - 1);
  const { firstColumn = 0, lastColumn = WEEK - 1 } = options;
  const taken: boolean[][] = Array.from({ length: options.lanes }, () => []);
  const hidden = Array.from({ length: WEEK }, () => 0);
  const placed: Placed[] = [];

  const laneFor = (first: number, last: number): number =>
    taken.findIndex((lane) => {
      for (let column = first; column <= last; column++) if (lane[column]) return false;
      for (let column = first; column <= last; column++) lane[column] = true;
      return true;
    });

  const inWeek = items
    .filter((item) => dayOf(item.start) <= sunday && dayOf(item.end) >= monday)
    .sort((a, b) => (a.start < b.start ? -1 : 1));

  for (const item of inWeek.filter((candidate) => candidate.kind === 'range')) {
    const first = Math.max(firstColumn, daysBetween(monday, dayOf(item.start)));
    const last = Math.min(lastColumn, daysBetween(monday, dayOf(item.end)));
    if (first > last) continue;
    const lane = laneFor(first, last);
    if (lane < 0) {
      for (let column = first; column <= last; column++) hidden[column] = (hidden[column] ?? 0) + 1;
      continue;
    }
    const gone = (daysBetween(addDays(monday, first), today) + 0.5) / (last - first + 1);
    placed.push({
      item,
      first,
      last,
      lane,
      opens: dayOf(item.start) >= monday,
      closes: dayOf(item.end) <= sunday,
      elapsed: Math.min(1, Math.max(0, gone)),
      over: dayOf(item.end) < today,
    });
  }

  if (options.events) {
    for (const item of inWeek.filter((candidate) => candidate.kind === 'event')) {
      const column = daysBetween(monday, dayOf(item.start));
      const lane = laneFor(column, column);
      if (lane < 0) hidden[column] = (hidden[column] ?? 0) + 1;
      else
        placed.push({
          item,
          first: column,
          last: column,
          lane,
          opens: true,
          closes: true,
          elapsed: 0,
          over: false,
        });
    }
  }
  return { placed, hidden };
}

const hourOf = (moment: string): number => {
  const [hour = 0, minute = 0] = timeOf(moment).split(':').map(Number);
  return hour + minute / 60;
};

export const FIRST_HOUR = 8;
export const LAST_HOUR = 23;
const SHORTEST_HOURS = 0.5;

/** An event in the hours of the week view, as parts of the height and of its day's width. */
export interface Block {
  item: Item;
  column: number;
  top: number;
  height: number;
  /** Events at the same time share their day, side by side. */
  side: number;
  sides: number;
  /** Too low for two lines. */
  short: boolean;
}

/** What is outside the hours shown stays visible at their edge. */
export function placeHours(items: readonly Item[], monday: Day): Block[] {
  const span = LAST_HOUR - FIRST_HOUR;
  const blocks: Block[] = [];
  for (let column = 0; column < WEEK; column++) {
    const day = addDays(monday, column);
    const events = items
      .filter((item) => item.kind === 'event' && dayOf(item.start) === day)
      .sort((a, b) => (a.start < b.start ? -1 : 1));
    let cluster: Block[] = [];
    let clusterEnd = '';
    let ends: string[] = [];
    const close = () => {
      for (const block of cluster) block.sides = ends.length;
    };
    for (const item of events) {
      if (cluster.length > 0 && item.start >= clusterEnd) {
        close();
        cluster = [];
        ends = [];
      }
      const free = ends.findIndex((end) => end <= item.start);
      const side = free < 0 ? ends.length : free;
      ends[side] = item.end;
      clusterEnd = item.end > clusterEnd ? item.end : clusterEnd;
      const from = Math.min(LAST_HOUR - SHORTEST_HOURS, Math.max(FIRST_HOUR, hourOf(item.start)));
      const to = Math.min(LAST_HOUR, Math.max(from + SHORTEST_HOURS, hourOf(item.end)));
      const block: Block = {
        item,
        column,
        top: (from - FIRST_HOUR) / span,
        height: (to - from) / span,
        side,
        sides: 1,
        short: to - from < 1.5,
      };
      cluster.push(block);
      blocks.push(block);
    }
    close();
  }
  return blocks;
}
