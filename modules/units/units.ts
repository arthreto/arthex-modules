interface Scale {
  toBase: (value: number) => number;
  fromBase: (value: number) => number;
}

const factor = (size: number): Scale => ({
  toBase: (value) => value * size,
  fromBase: (value) => value / size,
});

const offset = (toBase: Scale['toBase'], fromBase: Scale['fromBase']): Scale => ({ toBase, fromBase });

/** The forms of a word, in lower case and without accents, as a query is compared. */
const forms = (...words: string[]) => words.flatMap((word) => [word, `${word}s`]);

/**
 * `words` are compared without regard to case or accents. `exact` are symbols that mean this unit
 * only as written: `B` is a byte where `b` could be a bit, `K` a kelvin where `k` is a thousand.
 */
export const CATEGORIES = [
  {
    id: 'length',
    from: 'meter',
    units: [
      {
        id: 'millimeter',
        scale: factor(0.001),
        words: ['mm', ...forms('millimetre', 'millimeter')],
      },
      { id: 'centimeter', scale: factor(0.01), words: ['cm', ...forms('centimetre', 'centimeter')] },
      { id: 'meter', scale: factor(1), words: ['m', ...forms('metre', 'meter')] },
      { id: 'kilometer', scale: factor(1000), words: ['km', ...forms('kilometre', 'kilometer')] },
      { id: 'inch', scale: factor(0.0254), words: ['po', 'in', 'inch', 'inches', ...forms('pouce')] },
      { id: 'foot', scale: factor(0.3048), words: ['ft', 'foot', 'feet', ...forms('pied')] },
      { id: 'yard', scale: factor(0.9144), words: ['yd', ...forms('yard')] },
      { id: 'mile', scale: factor(1609.344), words: ['mi', ...forms('mile')] },
    ],
  },
  {
    id: 'area',
    from: 'squareMeter',
    units: [
      { id: 'squareCentimeter', scale: factor(0.0001), words: ['cm²', 'cm2'] },
      { id: 'squareMeter', scale: factor(1), words: ['m²', 'm2'] },
      { id: 'hectare', scale: factor(10000), words: ['ha', ...forms('hectare')] },
      { id: 'squareKilometer', scale: factor(1000000), words: ['km²', 'km2'] },
      { id: 'squareFoot', scale: factor(0.09290304), words: ['ft²', 'ft2'] },
      { id: 'acre', scale: factor(4046.8564224), words: ['ac', ...forms('acre')] },
    ],
  },
  {
    id: 'volume',
    from: 'liter',
    units: [
      { id: 'milliliter', scale: factor(0.001), words: ['ml', ...forms('millilitre', 'milliliter')] },
      { id: 'centiliter', scale: factor(0.01), words: ['cl', ...forms('centilitre', 'centiliter')] },
      { id: 'liter', scale: factor(1), words: ['l', ...forms('litre', 'liter')] },
      { id: 'cubicMeter', scale: factor(1000), words: ['m³', 'm3'] },
      { id: 'gallon', scale: factor(3.785411784), words: ['gal', ...forms('gallon')] },
    ],
  },
  {
    id: 'mass',
    from: 'kilogram',
    units: [
      { id: 'milligram', scale: factor(0.000001), words: ['mg', ...forms('milligramme', 'milligram')] },
      { id: 'gram', scale: factor(0.001), words: ['g', ...forms('gramme', 'gram')] },
      { id: 'kilogram', scale: factor(1), words: ['kg', ...forms('kilo', 'kilogramme', 'kilogram')] },
      { id: 'tonne', scale: factor(1000), words: ['t', ...forms('tonne')] },
      { id: 'ounce', scale: factor(0.028349523125), words: ['oz', ...forms('once', 'ounce')] },
      { id: 'pound', scale: factor(0.45359237), words: ['lb', 'lbs', ...forms('livre', 'pound')] },
    ],
  },
  {
    id: 'temperature',
    from: 'celsius',
    units: [
      { id: 'celsius', scale: factor(1), words: ['c', '°c', 'celsius'] },
      {
        id: 'fahrenheit',
        scale: offset(
          (value) => ((value - 32) * 5) / 9,
          (value) => (value * 9) / 5 + 32,
        ),
        words: ['f', '°f', 'fahrenheit'],
      },
      {
        id: 'kelvin',
        scale: offset(
          (value) => value - 273.15,
          (value) => value + 273.15,
        ),
        exact: ['K'],
        words: ['kelvin'],
      },
    ],
  },
  {
    id: 'speed',
    from: 'kilometerPerHour',
    units: [
      { id: 'kilometerPerHour', scale: factor(1 / 3.6), words: ['km/h', 'kmh', 'kph'] },
      { id: 'meterPerSecond', scale: factor(1), words: ['m/s'] },
      { id: 'milePerHour', scale: factor(0.44704), words: ['mph'] },
      { id: 'knot', scale: factor(1852 / 3600), words: ['nd', 'kn', ...forms('noeud', 'nœud', 'knot')] },
    ],
  },
  {
    id: 'time',
    from: 'minute',
    units: [
      { id: 'millisecond', scale: factor(0.001), words: ['ms', ...forms('milliseconde', 'millisecond')] },
      { id: 'second', scale: factor(1), words: ['s', 'sec', ...forms('seconde', 'second')] },
      { id: 'minute', scale: factor(60), words: ['min', ...forms('minute')] },
      { id: 'hour', scale: factor(3600), words: ['h', 'hr', ...forms('heure', 'hour')] },
      { id: 'day', scale: factor(86400), words: ['j', 'd', ...forms('jour', 'day')] },
      { id: 'week', scale: factor(604800), words: ['sem', 'wk', ...forms('semaine', 'week')] },
      { id: 'year', scale: factor(31557600), words: ['an', 'ans', 'yr', ...forms('annee', 'year')] },
    ],
  },
  {
    id: 'data',
    from: 'megabyte',
    units: [
      { id: 'bit', scale: factor(0.125), words: [...forms('bit')] },
      { id: 'byte', scale: factor(1), exact: ['B'], words: ['o', ...forms('octet', 'byte')] },
      { id: 'kilobyte', scale: factor(1e3), words: ['ko', 'kb', ...forms('kilooctet', 'kilobyte')] },
      { id: 'megabyte', scale: factor(1e6), words: ['mo', 'mb', ...forms('megaoctet', 'megabyte')] },
      { id: 'gigabyte', scale: factor(1e9), words: ['go', 'gb', ...forms('gigaoctet', 'gigabyte')] },
      { id: 'terabyte', scale: factor(1e12), words: ['to', 'tb', ...forms('teraoctet', 'terabyte')] },
      { id: 'kibibyte', scale: factor(1024), words: ['kio', 'kib', ...forms('kibioctet', 'kibibyte')] },
      { id: 'mebibyte', scale: factor(1024 ** 2), words: ['mio', 'mib', ...forms('mebioctet', 'mebibyte')] },
      { id: 'gibibyte', scale: factor(1024 ** 3), words: ['gio', 'gib', ...forms('gibioctet', 'gibibyte')] },
    ],
  },
] as const;

export type Category = (typeof CATEGORIES)[number];
export type CategoryId = Category['id'];
export type Unit = Category['units'][number];
export type UnitId = Unit['id'];

export const categoryOf = (unit: Unit): Category =>
  CATEGORIES.find((category) => category.units.some((other) => other.id === unit.id)) ?? CATEGORIES[0];

/** The unit a category starts on. */
export const startingUnit = (category: Category): Unit =>
  category.units.find((unit) => unit.id === category.from) ?? category.units[0];

export const convert = (value: number, from: Unit, to: Unit): number =>
  to.scale.fromBase(from.scale.toBase(value));

/**
 * The other units of the category, the easiest to read first: the closer a result is to a number
 * of a few digits, the higher it comes.
 */
export function ranked(value: number, from: Unit): { unit: Unit; value: number }[] {
  const easeOf = (result: number) => Math.abs(Math.log10(Math.abs(result) || 1) - 1);
  return (categoryOf(from).units as readonly Unit[])
    .filter((unit) => unit.id !== from.id)
    .map((unit) => ({ unit, value: convert(value, from, unit) }))
    .sort((a, b) => easeOf(a.value) - easeOf(b.value));
}

/** At most 8 significant digits: conversions rarely end, and no reader needs more. */
export function formatValue(value: number, locale: string, useGrouping = true): string {
  return new Intl.NumberFormat(locale, { maximumSignificantDigits: 8, useGrouping }).format(value);
}
