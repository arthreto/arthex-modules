const DAY = /^(\d{4})(\d{2})(\d{2})(?:T(\d{2})(\d{2})(\d{2})?(Z)?)?$/;

/** A date of a calendar file (`20261120` or `20261120T200000`, `Z` for UTC) as people read it. */
export function dayOf(day: string, locale: string): string {
  const parts = DAY.exec(day.trim());
  if (!parts) return day;
  const [year = 0, month = 1, date = 1, hours = 0, minutes = 0, seconds = 0] = parts
    .slice(1, 7)
    .map((part) => Number(part ?? 0));
  const when = parts[7]
    ? new Date(Date.UTC(year, month - 1, date, hours, minutes, seconds))
    : new Date(year, month - 1, date, hours, minutes, seconds);
  return when.toLocaleString(
    locale,
    parts[4] ? { dateStyle: 'long', timeStyle: 'short' } : { dateStyle: 'long' },
  );
}

/** Calendars open `.ics` files, which hold their events in a `VCALENDAR`. */
export function calendarOf(content: string): string {
  const value = content.trim();
  if (/^BEGIN:VCALENDAR/i.test(value)) return value;
  return ['BEGIN:VCALENDAR', 'VERSION:2.0', 'PRODID:-//Arthex//QR Code//EN', value, 'END:VCALENDAR'].join(
    '\r\n',
  );
}
