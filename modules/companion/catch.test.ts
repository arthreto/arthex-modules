import { describe, expect, it } from 'vitest';
import { clampedX, isOver, secondsLeft, startCatch, step, TICK_MS, CATCH_SECONDS } from './catch';

const falling = (x: number, y: number, trap = false) => ({ id: 1, x, y, trap, turn: 0 });

describe('catch', () => {
  it('catches what falls on the mascot', () => {
    const next = step({ ticks: 0, score: 0, items: [falling(50, 79)] }, 50);
    expect(next.score).toBe(1);
    expect(next.items).toEqual([]);
  });

  it('lets fall what is too far away, until it leaves the field', () => {
    const near = step({ ticks: 0, score: 0, items: [falling(10, 79)] }, 50);
    expect(near.score).toBe(0);
    expect(near.items).toHaveLength(1);
    expect(step({ ticks: 0, score: 0, items: [falling(10, 99)] }, 50).items).toEqual([]);
  });

  it('leaves alone what is still high up', () => {
    expect(step({ ticks: 0, score: 0, items: [falling(50, 10)] }, 50).score).toBe(0);
  });

  it('loses points on a trap, without going below zero', () => {
    expect(step({ ticks: 0, score: 5, items: [falling(50, 79, true)] }, 50).score).toBe(3);
    expect(step({ ticks: 0, score: 1, items: [falling(50, 79, true)] }, 50).score).toBe(0);
  });

  it('brings new items, some of them traps', () => {
    let state = startCatch();
    const seen: boolean[] = [];
    for (let tick = 0; tick < 200; tick += 1) {
      state = step(state, -100);
      for (const item of state.items) {
        expect(item.x).toBeGreaterThanOrEqual(8);
        expect(item.x).toBeLessThan(92);
        if (item.id === state.ticks) seen.push(item.trap);
      }
    }
    expect(seen.length).toBeGreaterThan(10);
    expect(seen).toContain(true);
    expect(seen).toContain(false);
  });

  it('lasts as long as it says', () => {
    let state = startCatch();
    expect(secondsLeft(state)).toBe(CATCH_SECONDS);
    const ticks = (CATCH_SECONDS * 1000) / TICK_MS;
    for (let tick = 0; tick < ticks - 1; tick += 1) state = step(state, 0);
    expect(isOver(state)).toBe(false);
    expect(isOver(step(state, 0))).toBe(true);
  });

  it('keeps the mascot on the field', () => {
    expect(clampedX(-20)).toBe(6);
    expect(clampedX(50)).toBe(50);
    expect(clampedX(120)).toBe(94);
  });
});
