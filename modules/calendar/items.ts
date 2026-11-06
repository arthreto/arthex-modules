import { addDays, daysBetween, type Day } from './dates';

export const ITEM_TINTS = ['sky', 'violet', 'green', 'orange', 'pink', 'red'] as const;
export type ItemTint = (typeof ITEM_TINTS)[number];
export type ItemKind = 'event' | 'range';

export interface Item {
  id: string;
  kind: ItemKind;
  title: string;
  tint: ItemTint;
  /** `YYYY-MM-DDTHH:MM`, local: it sorts as text. */
  start: string;
  end: string;
  /** Minutes before the event, or before the end of the deadline. */
  reminder: number | null;
}

export const dayOf = (moment: string): Day => moment.slice(0, 10);
export const timeOf = (moment: string): string => moment.slice(11);

export const REMINDERS: Record<ItemKind, readonly (number | null)[]> = {
  event: [null, 0, 10, 60, 1440],
  range: [null, 60, 1440, 4320],
};

/** What the editor shows: an event has one date, a deadline has two, and switching keeps both. */
export interface Draft {
  id: string;
  kind: ItemKind;
  title: string;
  tint: ItemTint;
  date: Day;
  startTime: string;
  endTime: string;
  from: Day;
  fromTime: string;
  to: Day;
  toTime: string;
  reminder: number | null;
}

export const defaultReminder = (kind: ItemKind): number => (kind === 'range' ? 1440 : 10);

export function newDraft(kind: ItemKind, day: Day): Draft {
  return {
    id: '',
    kind,
    title: '',
    tint: 'sky',
    date: day,
    startTime: '10:00',
    endTime: '11:00',
    from: day,
    fromTime: '09:00',
    to: addDays(day, 7),
    toTime: '18:00',
    reminder: defaultReminder(kind),
  };
}

export function draftOf(item: Item): Draft {
  const draft = newDraft(item.kind, dayOf(item.start));
  const place =
    item.kind === 'event'
      ? { startTime: timeOf(item.start), endTime: timeOf(item.end) }
      : { fromTime: timeOf(item.start), to: dayOf(item.end), toTime: timeOf(item.end) };
  return { ...draft, ...place, id: item.id, title: item.title, tint: item.tint, reminder: item.reminder };
}

/** What the draft saves, or nothing while it is not valid. */
export function itemOf(draft: Draft, id: string): Item | null {
  const [start, end] =
    draft.kind === 'event'
      ? [`${draft.date}T${draft.startTime}`, `${draft.date}T${draft.endTime}`]
      : [`${draft.from}T${draft.fromTime}`, `${draft.to}T${draft.toTime}`];
  if (draft.title.trim() === '' || end <= start) return null;
  return {
    id,
    kind: draft.kind,
    title: draft.title.trim(),
    tint: draft.tint,
    start,
    end,
    reminder: draft.reminder,
  };
}

export const isBackwards = (draft: Draft): boolean =>
  draft.title.trim() !== '' && itemOf(draft, draft.id) === null;

export const coversDay = (item: Item, day: Day): boolean =>
  dayOf(item.start) <= day && dayOf(item.end) >= day;

/** How far a deadline has gone today, from `0` to `1`, counting today as half gone. */
export function progressOf(item: Item, today: Day): number {
  const length = daysBetween(dayOf(item.start), dayOf(item.end)) + 1;
  return Math.min(1, Math.max(0, (daysBetween(dayOf(item.start), today) + 0.5) / length));
}

export type Remaining =
  | { state: 'over' }
  | { state: 'endsToday' }
  | { state: 'starts'; count: number }
  | { state: 'left'; count: number };

export function remainingOf(item: Item, today: Day): Remaining {
  const left = daysBetween(today, dayOf(item.end));
  if (left < 0) return { state: 'over' };
  if (left === 0) return { state: 'endsToday' };
  const wait = daysBetween(today, dayOf(item.start));
  return wait > 0 ? { state: 'starts', count: wait } : { state: 'left', count: left };
}

/** What is coming: the next events and the deadline that ends first. */
export function comingUp(items: readonly Item[], now: string, today: Day, eventCount: number) {
  const events = items
    .filter((item) => item.kind === 'event' && item.end > now)
    .sort((a, b) => (a.start < b.start ? -1 : 1))
    .slice(0, eventCount);
  const deadline = items
    .filter((item) => item.kind === 'range' && coversDay(item, today))
    .sort((a, b) => (a.end < b.end ? -1 : 1))[0];
  return { events, deadline };
}
