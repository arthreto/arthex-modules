import { describe, expect, it } from 'vitest';
import { CATCH_SECONDS, clampedX, isOver, secondsLeft, startCatch, step, type CatchState } from './catch';

const FRAME_MS = 16;
const falling = (x: number, y: number, trap = false) => ({ id: 1, x, y, trap, turn: 0, speed: 1 });
const withItems = (score: number, items: CatchState['items']): CatchState => ({
  elapsed: 0,
  score,
  items,
  nextAt: 100_000,
  nextId: 2,
});

/** The same numbers each time, so that a test never depends on luck. */
function seeded(seed: number): () => number {
  let value = seed;
  return () => {
    value = (value * 1664525 + 1013904223) % 4294967296;
    return value / 4294967296;
  };
}

/** Every item of a whole round, with the mascot out of the way. */
function round(random: () => number) {
  let state = startCatch(random);
  const seen = new Map<number, CatchState['items'][number]>();
  const firstSeen = new Map<number, number>();
  while (!isOver(state)) {
    state = step(state, -100, FRAME_MS, random);
    for (const item of state.items) {
      if (!seen.has(item.id)) {
        seen.set(item.id, item);
        firstSeen.set(item.id, state.elapsed);
      }
    }
  }
  return { items: [...seen.values()], times: [...firstSeen.values()] };
}

describe('catch', () => {
  it('catches what falls on the mascot', () => {
    const next = step(withItems(0, [falling(50, 79)]), 50, 40);
    expect(next.score).toBe(1);
    expect(next.items).toEqual([]);
  });

  it('lets fall what is too far away, until it leaves the field', () => {
    const near = step(withItems(0, [falling(10, 79)]), 50, 40);
    expect(near.score).toBe(0);
    expect(near.items).toHaveLength(1);
    expect(step(withItems(0, [falling(10, 99)]), 50, 40).items).toEqual([]);
  });

  it('leaves alone what is still high up', () => {
    expect(step(withItems(0, [falling(50, 10)]), 50, 40).score).toBe(0);
  });

  it('loses points on a trap, without going below zero', () => {
    expect(step(withItems(5, [falling(50, 79, true)]), 50, 40).score).toBe(3);
    expect(step(withItems(1, [falling(50, 79, true)]), 50, 40).score).toBe(0);
  });

  it('falls just as fast on any screen', () => {
    const after = (frames: number, ms: number): number => {
      let state = withItems(0, [falling(10, 0)]);
      for (let frame = 0; frame < frames; frame += 1) state = step(state, 50, ms);
      return state.items[0]?.y ?? -1;
    };
    expect(after(30, 1000 / 60)).toBeCloseTo(after(72, 1000 / 144), 5);
    expect(after(30, 1000 / 60)).toBeCloseTo(32.5, 5);
  });

  it('keeps what comes on the field', () => {
    for (const item of round(seeded(1)).items) {
      expect(item.x).toBeGreaterThanOrEqual(8);
      expect(item.x).toBeLessThan(92);
      expect(item.speed).toBeGreaterThanOrEqual(0.85);
      expect(item.speed).toBeLessThan(1.2);
    }
  });

  it('never follows the same pattern twice', () => {
    const places = (seed: number) => round(seeded(seed)).items.map((item) => item.x);
    expect(places(1)).not.toEqual(places(2));
    expect(new Set(places(1).map((x) => Math.round(x))).size).toBeGreaterThan(15);
  });

  it('brings items at uneven moments', () => {
    const { times } = round(seeded(3));
    const gaps = times.slice(1).map((time, index) => time - (times[index] ?? 0));
    expect(new Set(gaps.map((gap) => Math.round(gap / 50))).size).toBeGreaterThan(5);
    expect(Math.min(...gaps)).toBeGreaterThanOrEqual(250);
  });

  it('has traps among many more treats, not at a fixed rank', () => {
    const { items } = round(seeded(4));
    const traps = items.filter((item) => item.trap).length;
    expect(items.length).toBeGreaterThan(25);
    expect(traps).toBeGreaterThan(2);
    expect(traps).toBeLessThan(items.length / 2);
    const ranks = items.flatMap((item, rank) => (item.trap ? [rank] : []));
    const spaces = ranks.slice(1).map((rank, index) => rank - (ranks[index] ?? 0));
    expect(new Set(spaces).size).toBeGreaterThan(1);
  });

  it('lasts as long as it says', () => {
    let state = startCatch();
    expect(secondsLeft(state)).toBe(CATCH_SECONDS);
    state = step(state, 0, CATCH_SECONDS * 1000 - 1);
    expect(isOver(state)).toBe(false);
    expect(isOver(step(state, 0, 1))).toBe(true);
  });

  it('keeps the mascot on the field', () => {
    expect(clampedX(-20)).toBe(6);
    expect(clampedX(50)).toBe(50);
    expect(clampedX(120)).toBe(94);
  });
});
