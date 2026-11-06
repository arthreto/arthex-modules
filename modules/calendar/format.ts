import { getLocale } from '@arthex/sdk';
import { addMonths, addDays, dateOf, mondayOf, monthOf, type Day, type Level } from './dates';
import { t } from './i18n';
import { dayOf, remainingOf, timeOf, type Item } from './items';

const capitalized = (text: string) => text.charAt(0).toLocaleUpperCase() + text.slice(1);

const format = (day: Day, options: Intl.DateTimeFormatOptions) =>
  new Intl.DateTimeFormat(getLocale(), options).format(dateOf(day));

export const longDay = (day: Day) =>
  capitalized(format(day, { weekday: 'long', day: 'numeric', month: 'long' }));
export const shortDay = (day: Day) => format(day, { day: 'numeric', month: 'short' });
export const monthName = (day: Day, withYear = true) =>
  capitalized(format(day, withYear ? { month: 'long', year: 'numeric' } : { month: 'long' }));

/** Monday first, whatever the language. */
export function weekdayNames(): string[] {
  const monday = '2024-01-01';
  return Array.from({ length: 7 }, (_, index) => format(addDays(monday, index), { weekday: 'short' }));
}

export const weekdayName = (day: Day) => capitalized(format(day, { weekday: 'short' }));

export function timeText(moment: string): string {
  const [hour = '0', minute = '00'] = timeOf(moment).split(':');
  const shown = String(Number(hour));
  return minute === '00' ? t('time.hour', { hour: shown }) : t('time.minute', { hour: shown, minute });
}

export const hourText = (hour: number) => t('time.hour', { hour: String(hour) });

export function periodTitle(level: Level, focus: Day): string {
  const year = focus.slice(0, 4);
  switch (level) {
    case 'year':
      return year;
    case 'month':
      return monthName(focus);
    case 'quarter': {
      const last = addMonths(monthOf(focus), 2);
      const sameYear = last.slice(0, 4) === year;
      const lastName = format(last, { month: 'long', year: 'numeric' });
      return `${monthName(focus, !sameYear)} – ${lastName}`;
    }
    case 'week': {
      const monday = mondayOf(focus);
      return t('period.week', {
        from: format(monday, { day: 'numeric' }),
        to: format(addDays(monday, 6), { day: 'numeric', month: 'long', year: 'numeric' }),
      });
    }
  }
}

export function remainingText(item: Item, today: Day): string {
  const remaining = remainingOf(item, today);
  switch (remaining.state) {
    case 'over':
      return t('range.over');
    case 'endsToday':
      return t('range.endsToday');
    case 'starts':
      return t('range.starts', { count: remaining.count });
    case 'left':
      return t('range.left', { count: remaining.count });
  }
}

export const rangeSpan = (item: Item) =>
  t('range.span', { from: shortDay(dayOf(item.start)), to: shortDay(dayOf(item.end)) });

/** What a bar says when it is too short to say it, and to a screen reader. */
export function itemDescription(item: Item): string {
  return item.kind === 'event'
    ? t('item.at', { title: item.title, from: timeText(item.start), to: timeText(item.end) })
    : t('item.rangeAt', {
        title: item.title,
        from: shortDay(dayOf(item.start)),
        fromTime: timeText(item.start),
        to: shortDay(dayOf(item.end)),
        toTime: timeText(item.end),
      });
}
