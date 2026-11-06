/** What the Flair game shows of its words. */

export interface TriedWord {
  word: string;
  temperature: number;
}

const HOT = 50;
const WARM = 25;
const COOL = 5;
const BAR_OFFSET = 10;
const BAR_SMALLEST = 2;
const BAR_LARGEST = 100;

export type Heat = 'hot' | 'warm' | 'cool' | 'cold';

export function heatOf(temperature: number): Heat {
  if (temperature >= HOT) return 'hot';
  if (temperature >= WARM) return 'warm';
  if (temperature >= COOL) return 'cool';
  return 'cold';
}

export const barWidth = (temperature: number): number =>
  Math.max(BAR_SMALLEST, Math.min(BAR_LARGEST, temperature + BAR_OFFSET));

export const isExcited = (temperature: number | null): boolean => temperature !== null && temperature >= HOT;

/** The warmest first, whatever the order they were tried in. */
export const warmestFirst = (words: readonly TriedWord[]): TriedWord[] =>
  [...words].sort((a, b) => b.temperature - a.temperature);

export const normalized = (word: string): string => word.trim().toLowerCase();

export const alreadyTried = (words: readonly TriedWord[], word: string): boolean =>
  words.some((tried) => tried.word === normalized(word));

export const megabytes = (bytes: number): number => Math.round(bytes / 1e6);

/** A readable count: "1,4 million" is more than the player needs to know exactly. */
export const millions = (words: number): number => Math.round(words / 1e5) / 10;
