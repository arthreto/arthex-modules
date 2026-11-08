import { describe, expect, it } from 'vitest';
import { calendarOf, dayOf } from './calendar';

describe('calendar', () => {
  it('writes a day out, with its time when it has one', () => {
    expect(dayOf('20261120', 'fr')).toBe('20 novembre 2026');
    expect(dayOf('20261120T200000', 'en')).toBe('November 20, 2026 at 8:00 PM');
    expect(dayOf('soon', 'fr')).toBe('soon');
  });

  it('puts a lone event in a calendar', () => {
    expect(calendarOf('BEGIN:VEVENT\nEND:VEVENT').split('\r\n')[0]).toBe('BEGIN:VCALENDAR');
    expect(calendarOf('BEGIN:VCALENDAR\nEND:VCALENDAR')).toBe('BEGIN:VCALENDAR\nEND:VCALENDAR');
  });
});
