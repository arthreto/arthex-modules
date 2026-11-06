/** The rules of the "Catch" game, in percent of its field and in real time, whatever the screen's rate. */

export const CATCH_SECONDS = 20;
const FALL_PER_SECOND = 65;
const TURN_PER_SECOND = 125;
const CATCH_HEIGHT = 80;
const CATCH_REACH = 9;
const SHORTEST_GAP_MS = 300;
const LONGEST_GAP_MS = 820;
const TRAP_CHANCE = 0.25;
const TRAP_COST = 2;
const LEFT_MARGIN = 8;
const SPREAD = 84;
const SLOWEST = 0.85;
const FASTEST = 1.2;
const ENTRY_HEIGHT = -6;
const LEFTMOST = 6;
const RIGHTMOST = 94;

export type Random = () => number;

export interface FallingItem {
  id: number;
  x: number;
  y: number;
  trap: boolean;
  turn: number;
  /** How much faster or slower than usual it falls. */
  speed: number;
}

export interface CatchState {
  elapsed: number;
  score: number;
  items: FallingItem[];
  /** When the next item comes, and its number. */
  nextAt: number;
  nextId: number;
}

const between = (random: Random, low: number, high: number): number => low + random() * (high - low);

const nextGap = (random: Random): number => between(random, SHORTEST_GAP_MS, LONGEST_GAP_MS);

export const startCatch = (random: Random = Math.random): CatchState => ({
  elapsed: 0,
  score: 0,
  items: [],
  nextAt: nextGap(random),
  nextId: 0,
});

export const secondsLeft = (state: CatchState): number => CATCH_SECONDS - Math.floor(state.elapsed / 1000);

export const isOver = (state: CatchState): boolean => secondsLeft(state) <= 0;

export const clampedX = (x: number): number => Math.max(LEFTMOST, Math.min(RIGHTMOST, x));

/** `ms` later, with the mascot at `mascotX`. */
export function step(
  state: CatchState,
  mascotX: number,
  ms: number,
  random: Random = Math.random,
): CatchState {
  const elapsed = state.elapsed + ms;
  const seconds = ms / 1000;
  let { score, nextAt, nextId } = state;
  const items: FallingItem[] = [];
  for (const item of state.items) {
    const moved = {
      ...item,
      y: item.y + FALL_PER_SECOND * item.speed * seconds,
      turn: item.turn + TURN_PER_SECOND * seconds,
    };
    if (moved.y > CATCH_HEIGHT && Math.abs(moved.x - mascotX) < CATCH_REACH) {
      score = Math.max(0, score + (moved.trap ? -TRAP_COST : 1));
    } else if (moved.y < 100) {
      items.push(moved);
    }
  }
  while (elapsed >= nextAt) {
    items.push({
      id: nextId,
      x: LEFT_MARGIN + random() * SPREAD,
      y: ENTRY_HEIGHT,
      trap: random() < TRAP_CHANCE,
      turn: random() * 360,
      speed: between(random, SLOWEST, FASTEST),
    });
    nextId += 1;
    nextAt += nextGap(random);
  }
  return { elapsed, score, items, nextAt, nextId };
}
