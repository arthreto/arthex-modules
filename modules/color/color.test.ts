import { describe, expect, it } from 'vitest';
import { formats, fromHsv, parseColor, toHex, toHsv, type Color } from './color';
import en from './locales/en.json';
import fr from './locales/fr.json';
import { exactName, matchNames, NAMES } from './names';

const ACCENT: Color = { r: 76, g: 194, b: 255, a: 1 };

const valueOf = (color: Color, format: string) => formats(color).find((row) => row.format === format)?.value;

describe('parseColor', () => {
  it.each([
    ['#4CC2FF', ACCENT],
    ['#4cc2ff', ACCENT],
    ['4CC2FF', ACCENT],
    ['  #4CC2FF  ', ACCENT],
    ['#FFF', { r: 255, g: 255, b: 255, a: 1 }],
    ['#F00', { r: 255, g: 0, b: 0, a: 1 }],
    ['rgb(76, 194, 255)', ACCENT],
    ['RGB(76,194,255)', ACCENT],
    ['rgb(76 194 255)', ACCENT],
    ['rgba(76, 194, 255, 0.5)', { ...ACCENT, a: 0.5 }],
    ['rgb(76 194 255 / 50%)', { ...ACCENT, a: 0.5 }],
    ['76, 194, 255', ACCENT],
    ['76 194 255', ACCENT],
    ['hsl(0, 100%, 50%)', { r: 255, g: 0, b: 0, a: 1 }],
    ['hsla(120, 100%, 50%, 0.25)', { r: 0, g: 255, b: 0, a: 0.25 }],
    ['hsv(240, 100%, 100%)', { r: 0, g: 0, b: 255, a: 1 }],
    ['cmyk(0%, 100%, 100%, 0%)', { r: 255, g: 0, b: 0, a: 1 }],
  ])('reads %s', (text, expected) => {
    expect(parseColor(text)).toEqual(expected);
  });

  it('reads the alpha of a long code', () => {
    expect(parseColor('#4CC2FF80')?.a).toBeCloseTo(0.502, 2);
  });

  it.each([
    '',
    'hello',
    'facade',
    '123456',
    '12345',
    '#12',
    '#GGGGGG',
    'rgb(300, 0, 0)',
    'rgb(1, 2)',
    'rgb(a, b, c)',
    'foo(1, 2, 3)',
    '300, 0, 0',
    '2024 report',
    '12*8+3',
    '5 Mo',
  ])('refuses "%s"', (text) => {
    expect(parseColor(text)).toBeNull();
  });
});

describe('formats', () => {
  it('writes every format', () => {
    expect(formats(ACCENT).map((row) => row.format)).toEqual([
      'HEX',
      'RGB',
      'RGBA',
      'HSL',
      'HSLA',
      'HSV',
      'CMYK',
    ]);
    expect(valueOf(ACCENT, 'HEX')).toBe('#4CC2FF');
    expect(valueOf(ACCENT, 'RGB')).toBe('rgb(76, 194, 255)');
    expect(valueOf(ACCENT, 'RGBA')).toBe('rgba(76, 194, 255, 1)');
    expect(valueOf(ACCENT, 'HSL')).toBe('hsl(200, 100%, 65%)');
    expect(valueOf(ACCENT, 'HSV')).toBe('hsv(200, 70%, 100%)');
    expect(valueOf(ACCENT, 'CMYK')).toBe('cmyk(70%, 24%, 0%, 0%)');
  });

  it('handles black, white and grays', () => {
    const black = { r: 0, g: 0, b: 0, a: 1 };
    expect(valueOf(black, 'CMYK')).toBe('cmyk(0%, 0%, 0%, 100%)');
    expect(valueOf(black, 'HSL')).toBe('hsl(0, 0%, 0%)');
    expect(valueOf({ r: 255, g: 255, b: 255, a: 1 }, 'HSL')).toBe('hsl(0, 0%, 100%)');
    expect(valueOf({ r: 128, g: 128, b: 128, a: 1 }, 'HSV')).toBe('hsv(0, 0%, 50%)');
  });

  it('carries the alpha', () => {
    const faded = { ...ACCENT, a: 0.5 };
    expect(valueOf(faded, 'RGBA')).toBe('rgba(76, 194, 255, 0.5)');
    expect(valueOf(faded, 'HSLA')).toBe('hsla(200, 100%, 65%, 0.5)');
    expect(valueOf(faded, 'HEX')).toBe('#4CC2FF80');
  });

  it('reads back what it writes', () => {
    for (const row of formats(ACCENT)) {
      const back = parseColor(row.value);
      expect(back, row.format).not.toBeNull();
      expect(Math.abs((back?.r ?? 0) - ACCENT.r), row.value).toBeLessThanOrEqual(3);
      expect(Math.abs((back?.g ?? 0) - ACCENT.g), row.value).toBeLessThanOrEqual(3);
      expect(Math.abs((back?.b ?? 0) - ACCENT.b), row.value).toBeLessThanOrEqual(3);
    }
  });
});

describe('the wheel', () => {
  it.each([
    [0, 1, 1, '#FF0000'],
    [120, 1, 1, '#00FF00'],
    [240, 1, 1, '#0000FF'],
    [0, 0, 1, '#FFFFFF'],
    [200, 1, 0, '#000000'],
    [0, 0, 0.5, '#808080'],
  ])('hue %i, saturation %f, value %f gives %s', (h, s, v, hex) => {
    expect(toHex(fromHsv({ h, s, v }))).toBe(hex);
  });

  it('comes back to the same colour', () => {
    expect(fromHsv(toHsv(ACCENT))).toEqual(ACCENT);
  });

  it('keeps the alpha it is given', () => {
    expect(fromHsv({ h: 0, s: 1, v: 1 }, 0.4).a).toBe(0.4);
  });
});

describe('names', () => {
  const ids = (query: string) => matchNames(query).map((name) => name.id);

  it('knows both languages, without regard to case or accents', () => {
    expect(ids('rouge')[0]).toBe('red');
    expect(ids('RED')[0]).toBe('red');
    expect(ids('vert foret')[0]).toBe('forest');
    expect(exactName('Bleu ciel')?.hex).toBe('#87CEEB');
    expect(exactName('sky blue')?.hex).toBe('#87CEEB');
  });

  it('proposes the shades of a colour, the exact one first', () => {
    expect(ids('bleu')).toEqual(['blue', 'skyBlue', 'royalBlue', 'navy', 'steelBlue']);
  });

  it('also finds a colour by another of its words', () => {
    expect(ids('ciel')).toEqual(['skyBlue']);
  });

  it('takes short words only when they are exact', () => {
    expect(ids('or')).toEqual(['gold']);
    expect(ids('ro')).toEqual([]);
    expect(ids('ros')).toContain('pink');
  });

  it('stays out of other searches', () => {
    expect(ids('')).toEqual([]);
    expect(ids('rapport')).toEqual([]);
    expect(ids('discord')).toEqual([]);
  });

  it('gives every name a valid code', () => {
    for (const name of NAMES) {
      const color = parseColor(name.hex);
      expect(color, name.id).not.toBeNull();
      expect(color && toHex(color)).toBe(name.hex);
    }
  });
});

describe('locales', () => {
  const keysOf = (tree: object, prefix = ''): string[] =>
    Object.entries(tree).flatMap(([key, value]: [string, unknown]) =>
      typeof value === 'object' && value !== null ? keysOf(value, `${prefix}${key}.`) : [`${prefix}${key}`],
    );

  it('have the same keys in both languages', () => {
    expect(keysOf(fr).sort()).toEqual(keysOf(en).sort());
  });

  it('name every colour, and no other', () => {
    expect(Object.keys(en.names).sort()).toEqual(NAMES.map((name) => name.id).sort());
  });
});
