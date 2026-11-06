/** A local day as `YYYY-MM-DD`: it sorts as text, and needs no time zone. */
export type Day = string;

export const LEVELS = ['week', 'month', 'quarter', 'year'] as const;
export type Level = (typeof LEVELS)[number];

const pad = (value: number) => String(value).padStart(2, '0');

export const dayOfDate = (date: Date): Day =>
  `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`;

export function dateOf(day: Day): Date {
  const [year = 1970, month = 1, date = 1] = day.split('-').map(Number);
  return new Date(year, month - 1, date);
}

export function addDays(day: Day, count: number): Day {
  const date = dateOf(day);
  date.setDate(date.getDate() + count);
  return dayOfDate(date);
}

/** Days from one day to another. Through the calendar, as a day of a change of season is shorter. */
export function daysBetween(from: Day, to: Day): number {
  const [a, b] = [dateOf(from), dateOf(to)];
  return Math.round(
    (Date.UTC(b.getFullYear(), b.getMonth(), b.getDate()) -
      Date.UTC(a.getFullYear(), a.getMonth(), a.getDate())) /
      86_400_000,
  );
}

export const mondayOf = (day: Day): Day => addDays(day, -((dateOf(day).getDay() + 6) % 7));

export const monthOf = (day: Day): Day => `${day.slice(0, 7)}-01`;

export function addMonths(day: Day, count: number): Day {
  const date = dateOf(day);
  return dayOfDate(new Date(date.getFullYear(), date.getMonth() + count, 1));
}

export const yearOf = (day: Day): Day => `${day.slice(0, 4)}-01-01`;

/** The next scale when zooming out (`1`) or in (`-1`), which stops at both ends. */
export function zoomed(level: Level, direction: 1 | -1): Level {
  return LEVELS[LEVELS.indexOf(level) + direction] ?? level;
}

/** The day to show after moving one period forwards or backwards. */
export function moved(level: Level, focus: Day, direction: 1 | -1): Day {
  switch (level) {
    case 'week':
      return addDays(focus, 7 * direction);
    case 'month':
      return addMonths(focus, direction);
    case 'quarter':
      return addMonths(focus, 3 * direction);
    case 'year':
      return addMonths(focus, 12 * direction);
  }
}

/** The six weeks a month is drawn on: always the same number, so that nothing jumps. */
export function weeksOf(month: Day): Day[] {
  const first = mondayOf(monthOf(month));
  return Array.from({ length: 6 }, (_, index) => addDays(first, index * 7));
}
