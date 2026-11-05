import { describe, expect, it } from 'vitest';
import { fitted, isInZone, isZone, textOf, zoneBetween, type Word } from './words';

const word = (text: string, x: number, line: number): Word => ({ text, x, y: line * 30, w: 40, h: 20, line });

describe('zoneBetween', () => {
  it('works in every direction', () => {
    expect(zoneBetween({ x: 50, y: 40 }, { x: 10, y: 60 })).toEqual({ x: 10, y: 40, w: 40, h: 20 });
  });
});

describe('isZone', () => {
  it('takes a click for no zone', () => {
    expect(isZone({ x: 0, y: 0, w: 2, h: 2 })).toBe(false);
    expect(isZone({ x: 0, y: 0, w: 200, h: 2 })).toBe(false);
    expect(isZone({ x: 0, y: 0, w: 200, h: 30 })).toBe(true);
  });
});

describe('isInZone', () => {
  it('takes every word without a zone', () => {
    expect(isInZone(word('all', 500, 9), null)).toBe(true);
  });

  it('goes by the centre of the word', () => {
    const zone = { x: 0, y: 0, w: 30, h: 30 };
    expect(isInZone(word('half in', 0, 0), zone)).toBe(true);
    expect(isInZone(word('half out', 20, 0), zone)).toBe(false);
  });
});

describe('textOf', () => {
  it('keeps the lines of the image', () => {
    expect(textOf([word('one', 0, 0), word('two', 50, 0), word('three', 0, 1)])).toBe('one two\nthree');
  });

  it('is empty without words', () => {
    expect(textOf([])).toBe('');
  });
});

describe('fitted', () => {
  it('shrinks a large image to the room', () => {
    expect(fitted(2000, 1000, 500, 500)).toEqual({ width: 500, height: 250 });
  });

  it('enlarges a small image only a little', () => {
    expect(fitted(100, 50, 500, 500)).toEqual({ width: 160, height: 80 });
  });
});
