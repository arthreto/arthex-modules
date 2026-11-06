import { describe, expect, it } from 'vitest';
import type { Item } from './items';
import { placeHours, placeWeek } from './layout';

const MONDAY = '2026-11-02';
const event = (id: string, day: string, from = '10:00', to = '11:00'): Item => ({
  id,
  kind: 'event',
  title: id,
  tint: 'sky',
  start: `${day}T${from}`,
  end: `${day}T${to}`,
  reminder: null,
});
const deadline = (id: string, from: string, to: string): Item => ({
  ...event(id, from),
  kind: 'range',
  end: `${to}T18:00`,
});
const options = { lanes: 2, events: true };
const today = '2026-11-06';

describe('placeWeek', () => {
  it('keeps a deadline inside the week it crosses', () => {
    const { placed } = placeWeek([deadline('a', '2026-10-28', '2026-11-20')], MONDAY, today, options);
    expect(placed[0]).toMatchObject({ first: 0, last: 6, opens: false, closes: false });
  });

  it('starts and ends where the deadline does', () => {
    const { placed } = placeWeek([deadline('a', '2026-11-03', '2026-11-05')], MONDAY, today, options);
    expect(placed[0]).toMatchObject({ first: 1, last: 3, opens: true, closes: true });
  });

  it('leaves out what is in other weeks', () => {
    const items = [
      event('a', '2026-11-01'),
      event('b', '2026-11-09'),
      deadline('c', '2026-11-10', '2026-11-20'),
    ];
    expect(placeWeek(items, MONDAY, today, options).placed).toEqual([]);
  });

  it('stacks deadlines that overlap, and reuses a free line', () => {
    const { placed } = placeWeek(
      [
        deadline('a', '2026-11-02', '2026-11-04'),
        deadline('b', '2026-11-03', '2026-11-06'),
        deadline('c', '2026-11-05', '2026-11-08'),
      ],
      MONDAY,
      today,
      { lanes: 3, events: false },
    );
    expect(placed.map((part) => [part.item.id, part.lane])).toEqual([
      ['a', 0],
      ['b', 1],
      ['c', 0],
    ]);
  });

  it('puts events under the deadlines of their day', () => {
    const { placed } = placeWeek(
      [event('meeting', '2026-11-04'), deadline('report', '2026-11-02', '2026-11-08')],
      MONDAY,
      today,
      options,
    );
    expect(placed.map((part) => [part.item.id, part.lane])).toEqual([
      ['report', 0],
      ['meeting', 1],
    ]);
  });

  it('counts what does not fit, for each day', () => {
    const { placed, hidden } = placeWeek(
      [
        event('a', '2026-11-04'),
        event('b', '2026-11-04', '11:00', '12:00'),
        event('c', '2026-11-04', '12:00', '13:00'),
      ],
      MONDAY,
      today,
      options,
    );
    expect(placed).toHaveLength(2);
    expect(hidden).toEqual([0, 0, 1, 0, 0, 0, 0]);
  });

  it('counts a deadline that does not fit on each of its days', () => {
    const { hidden } = placeWeek(
      [deadline('a', '2026-11-02', '2026-11-08'), deadline('b', '2026-11-02', '2026-11-03')],
      MONDAY,
      today,
      { lanes: 1, events: false },
    );
    expect(hidden).toEqual([1, 1, 0, 0, 0, 0, 0]);
  });

  it('marks what has gone', () => {
    const { placed } = placeWeek([deadline('a', '2026-11-02', '2026-11-08')], MONDAY, today, options);
    expect(placed[0]?.elapsed).toBeCloseTo(4.5 / 7);
    expect(placed[0]?.over).toBe(false);
    const past = placeWeek([deadline('a', '2026-11-02', '2026-11-04')], MONDAY, today, options);
    expect(past.placed[0]).toMatchObject({ elapsed: 1, over: true });
    const coming = placeWeek([deadline('a', '2026-11-02', '2026-11-08')], MONDAY, '2026-10-01', options);
    expect(coming.placed[0]?.elapsed).toBe(0);
  });

  it('keeps a small month to its own days', () => {
    const items = [deadline('a', '2026-10-20', '2026-11-20')];
    const { placed } = placeWeek(items, '2026-10-26', today, {
      lanes: 2,
      events: false,
      firstColumn: 0,
      lastColumn: 5,
    });
    expect(placed[0]).toMatchObject({ first: 0, last: 5 });
    const none = placeWeek(items, '2026-10-26', today, {
      lanes: 2,
      events: false,
      firstColumn: 7,
      lastColumn: -1,
    });
    expect(none.placed).toEqual([]);
  });
});

describe('placeHours', () => {
  it('places an event by its hours', () => {
    const [block] = placeHours([event('a', '2026-11-03', '09:30', '11:00')], MONDAY);
    expect(block?.column).toBe(1);
    expect(block?.top).toBeCloseTo(1.5 / 15);
    expect(block?.height).toBeCloseTo(1.5 / 15);
    expect(block?.short).toBe(false);
  });

  it('keeps events outside the hours at their edge', () => {
    const [early, late] = placeHours(
      [event('early', '2026-11-03', '05:00', '06:00'), event('late', '2026-11-03', '23:30', '23:45')],
      MONDAY,
    );
    expect(early?.top).toBe(0);
    expect(early?.height).toBeCloseTo(0.5 / 15);
    expect(late?.top).toBeCloseTo(14.5 / 15);
    expect(late?.height).toBeCloseTo(0.5 / 15);
  });

  it('marks what is too low for two lines', () => {
    expect(placeHours([event('a', '2026-11-03', '10:00', '10:30')], MONDAY)[0]?.short).toBe(true);
  });

  it('shares the day between events at the same time', () => {
    const blocks = placeHours(
      [
        event('a', '2026-11-03', '10:00', '11:00'),
        event('b', '2026-11-03', '10:30', '12:00'),
        event('c', '2026-11-03', '11:00', '11:30'),
        event('d', '2026-11-03', '14:00', '15:00'),
      ],
      MONDAY,
    );
    expect(blocks.map((block) => [block.item.id, block.side, block.sides])).toEqual([
      ['a', 0, 2],
      ['b', 1, 2],
      ['c', 0, 2],
      ['d', 0, 1],
    ]);
  });

  it('ignores deadlines and other weeks', () => {
    const items = [deadline('a', '2026-11-03', '2026-11-05'), event('b', '2026-11-20')];
    expect(placeHours(items, MONDAY)).toEqual([]);
  });
});
