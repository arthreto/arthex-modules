/** The rules of the "Memory" game: 6 pairs, and never a wait to play the next card. */

export const PICTURES = [
  { icon: 'M12 20s-7-4.5-7-10a4 4 0 0 1 7-2.5A4 4 0 0 1 19 10c0 5.5-7 10-7 10Z', tint: 'pink' },
  {
    icon: 'M12 3l2.7 5.6 6.1.9-4.4 4.3 1 6.1L12 17l-5.4 2.9 1-6.1L3.2 9.5l6.1-.9Z',
    tint: 'amber',
  },
  { icon: 'M4 12a8 8 0 1 0 16 0 8 8 0 0 0-16 0ZM12 8v4l3 2', tint: 'sky' },
  { icon: 'M6 3h12l3 6-9 12L3 9ZM3 9h18', tint: 'violet' },
  { icon: 'M8 3h8v8a4 4 0 0 1-8 0ZM12 15v4M8 21h8', tint: 'orange' },
  { icon: 'M12 3a9 9 0 1 0 9 9 7 7 0 0 1-9-9Z', tint: 'green' },
] as const;

export interface MemoryState {
  /** Which picture each card holds. */
  cards: number[];
  open: number[];
  done: number[];
  moves: number;
}

/** Fisher-Yates, from a source of numbers in [0, 1[ that a test can choose. */
export function dealCards(random: () => number = Math.random): MemoryState {
  const cards = PICTURES.flatMap((_, picture) => [picture, picture]);
  for (let index = cards.length - 1; index > 0; index -= 1) {
    const other = Math.floor(random() * (index + 1));
    const held = cards[index] ?? 0;
    cards[index] = cards[other] ?? 0;
    cards[other] = held;
  }
  return { cards, open: [], done: [], moves: 0 };
}

/** A pair that matches is found at once. The next card puts away a pair that does not. */
export function flip(state: MemoryState, card: number): MemoryState {
  const ready = state.open.length >= 2 ? settle(state) : state;
  if (ready.open.includes(card) || ready.done.includes(card)) return ready;
  const open = [...ready.open, card];
  if (open.length < 2) return { ...ready, open };
  const [first, second] = open;
  const played = { ...ready, open, moves: ready.moves + 1 };
  return ready.cards[first ?? -1] === ready.cards[second ?? -1] ? settle(played) : played;
}

/** The two open cards stay found if they match, or turn back. */
export function settle(state: MemoryState): MemoryState {
  const [first, second] = state.open;
  if (first === undefined || second === undefined) return state;
  const same = state.cards[first] === state.cards[second];
  return { ...state, open: [], done: same ? [...state.done, first, second] : state.done };
}

export const isWon = (state: MemoryState): boolean => state.done.length === state.cards.length;
