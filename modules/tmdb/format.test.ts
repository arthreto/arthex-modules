import { describe, expect, it } from 'vitest';
import { ageOf, dayOf, durationOf, ratingOf, regionOf } from './format';

describe('format', () => {
  it('writes a day out and nothing for an unknown one', () => {
    expect(dayOf('2024-02-27', 'fr')).toBe('27 février 2024');
    expect(dayOf('2024-02-27', 'en')).toBe('February 27, 2024');
    expect(dayOf('', 'fr')).toBe('');
  });

  it('writes hours and minutes', () => {
    expect(durationOf(165)).toBe('2 h 45');
    expect(durationOf(120)).toBe('2 h 00');
    expect(durationOf(48)).toBe('48 min');
  });

  it('counts a year only once the birthday has passed', () => {
    expect(ageOf('1996-09-01', new Date(2026, 7, 31))).toBe(29);
    expect(ageOf('1996-09-01', new Date(2026, 8, 1))).toBe(30);
  });

  it('writes a rating with one decimal in the language', () => {
    expect(ratingOf(8.144, 'fr')).toBe('8,1');
    expect(ratingOf(7, 'en')).toBe('7.0');
  });

  it('asks TMDB in the language of the interface for the country of the system', () => {
    expect(regionOf('fr', 'fr-CA')).toEqual({ language: 'fr-CA', country: 'CA' });
    expect(regionOf('fr', 'fr')).toEqual({ language: 'fr-FR', country: 'FR' });
    expect(regionOf('en', 'en-GB')).toEqual({ language: 'en-GB', country: 'GB' });
  });
});
