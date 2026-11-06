import { describe, expect, it } from 'vitest';
import {
  alreadyTried,
  barWidth,
  heatOf,
  isExcited,
  megabytes,
  millions,
  normalized,
  warmestFirst,
} from './words';

describe('words', () => {
  it('says how warm a word is', () => {
    expect(heatOf(50)).toBe('hot');
    expect(heatOf(49.9)).toBe('warm');
    expect(heatOf(25)).toBe('warm');
    expect(heatOf(5)).toBe('cool');
    expect(heatOf(4.9)).toBe('cold');
    expect(heatOf(-30)).toBe('cold');
  });

  it('keeps the bar within its track', () => {
    expect(barWidth(-40)).toBe(2);
    expect(barWidth(30)).toBe(40);
    expect(barWidth(100)).toBe(100);
  });

  it('gets excited when close', () => {
    expect(isExcited(null)).toBe(false);
    expect(isExcited(49)).toBe(false);
    expect(isExcited(50)).toBe(true);
  });

  it('shows the warmest first and leaves the list as it is', () => {
    const words = [
      { word: 'table', temperature: -3 },
      { word: 'sucre', temperature: 58 },
      { word: 'lait', temperature: 34 },
    ];
    expect(warmestFirst(words).map(({ word }) => word)).toEqual(['sucre', 'lait', 'table']);
    expect(words[0]?.word).toBe('table');
  });

  it('recognises a word tried in another form', () => {
    expect(normalized('  Cacao ')).toBe('cacao');
    expect(alreadyTried([{ word: 'cacao', temperature: 1 }], ' CACAO')).toBe(true);
    expect(alreadyTried([{ word: 'cacao', temperature: 1 }], 'lait')).toBe(false);
  });

  it('rounds the sizes it shows', () => {
    expect(megabytes(612_345_678)).toBe(612);
    expect(millions(1_412_345)).toBe(1.4);
  });
});
