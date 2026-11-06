import { describe, expect, it } from 'vitest';
import { dealCards, flip, isWon, PICTURES, settle, type MemoryState } from './memory';

const small = (): MemoryState => ({ cards: [0, 1, 0, 1], open: [], done: [], moves: 0 });

describe('memory', () => {
  it('deals every picture twice', () => {
    const { cards } = dealCards();
    expect(cards).toHaveLength(PICTURES.length * 2);
    for (let picture = 0; picture < PICTURES.length; picture += 1) {
      expect(cards.filter((card) => card === picture)).toHaveLength(2);
    }
  });

  it('shuffles', () => {
    expect(dealCards(() => 0).cards).not.toEqual(dealCards(() => 0.99).cards);
  });

  it('keeps a pair that matches', () => {
    const state = settle(flip(flip(small(), 0), 2));
    expect(state.done).toEqual([0, 2]);
    expect(state.open).toEqual([]);
    expect(state.moves).toBe(1);
  });

  it('turns back a pair that does not match', () => {
    const state = settle(flip(flip(small(), 0), 1));
    expect(state.done).toEqual([]);
    expect(state.open).toEqual([]);
    expect(state.moves).toBe(1);
  });

  it('ignores a card already open or found, and a third one', () => {
    const one = flip(small(), 0);
    expect(flip(one, 0)).toBe(one);
    const two = flip(one, 1);
    expect(flip(two, 2)).toBe(two);
    const found = settle(flip(one, 2));
    expect(flip(found, 0)).toBe(found);
  });

  it('settles only once two are open', () => {
    const one = flip(small(), 0);
    expect(settle(one)).toBe(one);
  });

  it('is won when all are found', () => {
    let state = small();
    for (const pair of [
      [0, 2],
      [1, 3],
    ]) {
      expect(isWon(state)).toBe(false);
      state = settle(flip(flip(state, pair[0] ?? 0), pair[1] ?? 0));
    }
    expect(isWon(state)).toBe(true);
  });
});
