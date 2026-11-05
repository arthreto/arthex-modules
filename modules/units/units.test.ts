import { describe, expect, it } from 'vitest';
import en from './locales/en.json';
import fr from './locales/fr.json';
import { findUnit, parseNumber, parseQuery } from './parse';
import { CATEGORIES, categoryOf, convert, formatValue, ranked, startingUnit, type Unit } from './units';

const unit = (text: string): Unit => {
  const found = findUnit(text);
  if (!found) throw new Error(`no unit for ${text}`);
  return found;
};

const allUnits: readonly Unit[] = CATEGORIES.flatMap((category): readonly Unit[] => category.units);

describe('conversions', () => {
  it.each([
    ['km', 'm', 5, 5000],
    ['Mo', 'Ko', 5, 5000],
    ['Mio', 'Kio', 1, 1024],
    ['Go', 'Mio', 1, 953.67431640625],
    ['o', 'bit', 1, 8],
    ['in', 'cm', 1, 2.54],
    ['mi', 'km', 1, 1.609344],
    ['lb', 'kg', 1, 0.45359237],
    ['ha', 'm²', 1, 10000],
    ['l', 'ml', 2, 2000],
    ['km/h', 'm/s', 36, 10],
    ['h', 'min', 2, 120],
    ['j', 'h', 1, 24],
    ['°C', '°F', 100, 212],
    ['°F', '°C', 32, 0],
    ['°C', 'K', 0, 273.15],
    ['K', '°C', 0, -273.15],
  ])('%s to %s', (from, to, value, expected) => {
    expect(convert(value, unit(from), unit(to))).toBeCloseTo(expected, 8);
  });

  it('comes back to the same value', () => {
    for (const from of allUnits) {
      const to = startingUnit(categoryOf(from));
      expect(convert(convert(42, from, to), to, from)).toBeCloseTo(42, 6);
    }
  });
});

describe('ranked', () => {
  it('leaves the starting unit out and keeps its category', () => {
    const results = ranked(5, unit('Mo'));
    expect(results.map((result) => result.unit.id)).not.toContain('megabyte');
    expect(results).toHaveLength(8);
  });

  it('puts the easiest numbers to read first', () => {
    const results = ranked(30, unit('cm')).map((result) => result.unit.id);
    expect(results[0]).toBe('inch');
    expect(results.at(-1)).toBe('mile');
  });

  it('survives a value of zero', () => {
    expect(ranked(0, unit('km')).every((result) => result.value === 0)).toBe(true);
  });
});

describe('parseNumber', () => {
  it.each([
    ['5', 5],
    ['-5', -5],
    ['1,5', 1.5],
    ['1.5', 1.5],
    ['5,', 5],
    ['1 000', 1000],
    ['1 000,5', 1000.5],
    ['1,000.5', 1000.5],
    ['1.000,5', 1000.5],
    ['1.000.000', 1000000],
  ])('%s', (text, expected) => {
    expect(parseNumber(text)).toBe(expected);
  });

  it.each(['', 'abc', '5a', '1e3', ','])('refuses "%s"', (text) => {
    expect(parseNumber(text)).toBeNull();
  });
});

describe('findUnit', () => {
  it('ignores case and accents', () => {
    expect(unit('MO').id).toBe('megabyte');
    expect(unit('mo').id).toBe('megabyte');
    expect(unit('Mètres').id).toBe('meter');
    expect(unit('NŒUDS').id).toBe('knot');
  });

  it('knows the symbols of both languages', () => {
    expect(unit('po')).toBe(unit('in'));
    expect(unit('Mo')).toBe(unit('MB'));
    expect(unit('j')).toBe(unit('d'));
  });

  it('keeps apart what only case tells apart', () => {
    expect(unit('B').id).toBe('byte');
    expect(findUnit('b')).toBeNull();
    expect(unit('K').id).toBe('kelvin');
    expect(findUnit('k')).toBeNull();
  });

  it('never takes bits for bytes', () => {
    expect(findUnit('Mb')).toBeNull();
    expect(unit('MB').id).toBe('megabyte');
  });

  it('gives every word to a single unit', () => {
    const words = allUnits.flatMap((each): readonly string[] => each.words);
    expect(new Set(words).size).toBe(words.length);
  });
});

describe('parseQuery', () => {
  it.each([
    ['5 Mo', 5, 'megabyte'],
    ['5Mo', 5, 'megabyte'],
    ['1,5 km', 1.5, 'kilometer'],
    ['1 000 m', 1000, 'meter'],
    ['-5 °C', -5, 'celsius'],
    ['20°C', 20, 'celsius'],
    ['20 ° C', 20, 'celsius'],
    ['100 km/h', 100, 'kilometerPerHour'],
    ['30 m2', 30, 'squareMeter'],
    ['30 in', 30, 'inch'],
    ['  5 Mo  ', 5, 'megabyte'],
  ])('%s', (text, value, from) => {
    expect(parseQuery(text)).toMatchObject({ value, from: { id: from }, to: null });
  });

  it.each([
    ['5 Mo en Ko', 'kilobyte'],
    ['5 MB to KB', 'kilobyte'],
    ['30 cm en po', 'inch'],
    ['30 in in cm', 'centimeter'],
    ['20°C -> F', 'fahrenheit'],
    ['1 km → m', 'meter'],
  ])('%s has a target', (text, to) => {
    expect(parseQuery(text)?.to?.id).toBe(to);
  });

  it('shows everything while the target is still being typed', () => {
    expect(parseQuery('5 Mo en')).toMatchObject({ value: 5, from: { id: 'megabyte' }, to: null });
  });

  it.each([
    '',
    '5',
    'Mo',
    'hello',
    '5 hello',
    '12*8+3',
    '5 Mo en kg',
    '5 Mo en Mo',
    '5 Mo en truc',
    '5 Mo et 3',
  ])('refuses "%s"', (text) => {
    expect(parseQuery(text)).toBeNull();
  });
});

describe('formatValue', () => {
  it('follows the language', () => {
    expect(formatValue(1234.5, 'en')).toBe('1,234.5');
    expect(formatValue(1234.5, 'fr').replace(/\s/g, ' ')).toBe('1 234,5');
  });

  it('keeps 8 significant digits, even on small numbers', () => {
    expect(formatValue(1 / 3, 'en')).toBe('0.33333333');
    expect(formatValue(0.000000005, 'en')).toBe('0.000000005');
  });

  it('can leave out the grouping, for a value that is copied', () => {
    expect(formatValue(1234.5, 'en', false)).toBe('1234.5');
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

  it('name every category and unit', () => {
    for (const category of CATEGORIES) expect(en.categories).toHaveProperty(category.id);
    for (const each of allUnits) expect(en.units).toHaveProperty(each.id);
  });
});
