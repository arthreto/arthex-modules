import { describe, expect, it } from 'vitest';
import {
  comingUp,
  coversDay,
  draftOf,
  isBackwards,
  itemOf,
  newDraft,
  progressOf,
  remainingOf,
  type Item,
} from './items';

const event = (id: string, start: string, end: string): Item => ({
  id,
  kind: 'event',
  title: id,
  tint: 'sky',
  start,
  end,
  reminder: 10,
});
const deadline = (id: string, start: string, end: string): Item => ({
  ...event(id, start, end),
  kind: 'range',
});

describe('drafts', () => {
  it('starts on the chosen day, a deadline a week long', () => {
    const draft = newDraft('range', '2026-11-06');
    expect(draft.from).toBe('2026-11-06');
    expect(draft.to).toBe('2026-11-13');
    expect(draft.reminder).toBe(1440);
    expect(newDraft('event', '2026-11-06').reminder).toBe(10);
  });

  it('needs a title', () => {
    expect(itemOf(newDraft('event', '2026-11-06'), 'a')).toBeNull();
    expect(isBackwards(newDraft('event', '2026-11-06'))).toBe(false);
  });

  it('saves an event on its day', () => {
    const item = itemOf({ ...newDraft('event', '2026-11-06'), title: ' Meeting ' }, 'a');
    expect(item).toEqual({
      id: 'a',
      kind: 'event',
      title: 'Meeting',
      tint: 'sky',
      start: '2026-11-06T10:00',
      end: '2026-11-06T11:00',
      reminder: 10,
    });
  });

  it('saves a deadline from one date to another', () => {
    const draft = { ...newDraft('range', '2026-11-06'), title: 'Report', toTime: '20:00' };
    const item = itemOf(draft, 'a');
    expect(item?.start).toBe('2026-11-06T09:00');
    expect(item?.end).toBe('2026-11-13T20:00');
  });

  it('refuses an end before the start', () => {
    const draft = { ...newDraft('range', '2026-11-06'), title: 'Report', to: '2026-11-01' };
    expect(itemOf(draft, 'a')).toBeNull();
    expect(isBackwards(draft)).toBe(true);
    const sameTime = { ...newDraft('event', '2026-11-06'), title: 'Call', endTime: '10:00' };
    expect(itemOf(sameTime, 'a')).toBeNull();
  });

  it('gives back what an item holds', () => {
    const original = deadline('a', '2026-11-02T08:30', '2026-11-13T18:15');
    expect(itemOf(draftOf(original), 'a')).toEqual(original);
    const meeting = event('b', '2026-11-06T16:30', '2026-11-06T17:15');
    expect(itemOf(draftOf(meeting), 'b')).toEqual(meeting);
  });

  it('keeps an item without a reminder without one', () => {
    const silent = { ...event('a', '2026-11-06T10:00', '2026-11-06T11:00'), reminder: null };
    expect(draftOf(silent).reminder).toBeNull();
  });
});

describe('deadlines', () => {
  const report = deadline('a', '2026-11-02T09:00', '2026-11-13T18:00');

  it('covers its days', () => {
    expect(coversDay(report, '2026-11-02')).toBe(true);
    expect(coversDay(report, '2026-11-13')).toBe(true);
    expect(coversDay(report, '2026-11-14')).toBe(false);
  });

  it('says what is left', () => {
    expect(remainingOf(report, '2026-11-06')).toEqual({ state: 'left', count: 7 });
    expect(remainingOf(report, '2026-11-13')).toEqual({ state: 'endsToday' });
    expect(remainingOf(report, '2026-11-14')).toEqual({ state: 'over' });
    expect(remainingOf(report, '2026-10-30')).toEqual({ state: 'starts', count: 3 });
  });

  it('measures its progress, within its bounds', () => {
    expect(progressOf(report, '2026-10-01')).toBe(0);
    expect(progressOf(report, '2026-12-01')).toBe(1);
    expect(progressOf(report, '2026-11-02')).toBeCloseTo(0.5 / 12);
  });
});

describe('comingUp', () => {
  const items = [
    event('past', '2026-11-05T10:00', '2026-11-05T11:00'),
    event('later', '2026-11-08T10:00', '2026-11-08T11:00'),
    event('soon', '2026-11-06T16:00', '2026-11-06T17:00'),
    event('third', '2026-11-09T10:00', '2026-11-09T11:00'),
    deadline('long', '2026-11-01T09:00', '2026-11-30T18:00'),
    deadline('short', '2026-11-01T09:00', '2026-11-10T18:00'),
    deadline('next', '2026-11-20T09:00', '2026-11-21T18:00'),
  ];

  it('takes the next events, then the deadline that ends first', () => {
    const { events, deadline: first } = comingUp(items, '2026-11-06T12:00', '2026-11-06', 2);
    expect(events.map((item) => item.id)).toEqual(['soon', 'later']);
    expect(first?.id).toBe('short');
  });

  it('keeps an event that is going on', () => {
    const { events } = comingUp(items, '2026-11-06T16:30', '2026-11-06', 1);
    expect(events[0]?.id).toBe('soon');
  });

  it('has nothing when nothing is planned', () => {
    expect(comingUp([], '2026-11-06T12:00', '2026-11-06', 2)).toEqual({ events: [], deadline: undefined });
  });
});
