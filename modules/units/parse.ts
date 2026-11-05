import { CATEGORIES, categoryOf, type Unit } from './units';

/** Written in capitals, these are bits per convention: answering in bytes would be wrong by 8. */
const BITS = new Set(['Kb', 'Mb', 'Gb', 'Tb']);

const QUERY = /^(-?\d[\d\s.,]*?)\s*([^\d\s]\S*?)(?:\s+(?:en|in|to|vers|->|→)(?:\s+(\S+))?)?$/i;

const allUnits: readonly Unit[] = CATEGORIES.flatMap((category): readonly Unit[] => category.units);

const byWord = new Map<string, Unit>(
  allUnits.flatMap((unit) => unit.words.map((word): [string, Unit] => [word, unit])),
);

const fold = (text: string) =>
  text
    .normalize('NFD')
    .replace(/\p{Diacritic}/gu, '')
    .toLowerCase();

export function findUnit(text: string): Unit | null {
  if (BITS.has(text)) return null;
  const exact = allUnits.find((unit) => 'exact' in unit && (unit.exact as readonly string[]).includes(text));
  return exact ?? byWord.get(fold(text)) ?? null;
}

/**
 * "1 000,5", "1,000.5" and "1.5" all read as people write them: the last of two different
 * separators is the decimal one, and a separator that repeats groups thousands.
 */
export function parseNumber(text: string): number | null {
  const compact = text.replace(/\s/g, '');
  if (!/^-?\d+(?:[.,]\d*)*$/.test(compact)) return null;
  const separators = compact.match(/[.,]/g) ?? [];
  const last = separators.at(-1);
  const repeated =
    last !== undefined && separators.every((separator) => separator === last) && separators.length > 1;
  const decimal = last === undefined || repeated ? -1 : compact.lastIndexOf(last);
  const whole = decimal === -1 ? compact : compact.slice(0, decimal);
  const digits = whole.replace(/[.,]/g, '');
  const fraction = decimal === -1 ? '' : compact.slice(decimal + 1);
  const value = Number(`${digits}.${fraction}0`);
  return Number.isFinite(value) ? value : null;
}

export interface Conversion {
  value: number;
  from: Unit;
  to: Unit | null;
}

/** "5 Mo", "30 cm en po", "20°C in F". A word after "en" that is not a unit of the same kind is no conversion. */
export function parseQuery(text: string): Conversion | null {
  const match = QUERY.exec(text.trim().replace(/°\s+/, '°'));
  const [, number, unit, target] = match ?? [];
  if (number === undefined || unit === undefined) return null;
  const value = parseNumber(number);
  const from = findUnit(unit);
  if (value === null || from === null) return null;
  if (target === undefined) return { value, from, to: null };
  const to = findUnit(target);
  if (to === null || to.id === from.id || categoryOf(to).id !== categoryOf(from).id) return null;
  return { value, from, to };
}
