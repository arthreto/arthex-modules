import { describe, expect, it } from 'vitest';
import { addDays, addMonths, daysBetween, mondayOf, monthOf, moved, weeksOf, yearOf, zoomed } from './dates';

describe('days', () => {
  it('adds across months and years', () => {
    expect(addDays('2026-12-31', 1)).toBe('2027-01-01');
    expect(addDays('2026-03-01', -1)).toBe('2026-02-28');
  });

  it('counts through a change of season', () => {
    expect(daysBetween('2026-03-28', '2026-03-30')).toBe(2);
    expect(daysBetween('2026-10-24', '2026-10-26')).toBe(2);
    expect(daysBetween('2026-11-13', '2026-11-06')).toBe(-7);
  });

  it('finds the Monday of a week', () => {
    expect(mondayOf('2026-11-06')).toBe('2026-11-02');
    expect(mondayOf('2026-11-02')).toBe('2026-11-02');
    expect(mondayOf('2026-11-08')).toBe('2026-11-02');
    expect(mondayOf('2026-11-01')).toBe('2026-10-26');
  });
});

describe('months', () => {
  it('finds the first day', () => {
    expect(monthOf('2026-11-06')).toBe('2026-11-01');
    expect(yearOf('2026-11-06')).toBe('2026-01-01');
  });

  it('moves from a long month to a short one', () => {
    expect(addMonths('2026-01-31', 1)).toBe('2026-02-01');
    expect(addMonths('2026-11-06', 2)).toBe('2027-01-01');
    expect(addMonths('2026-01-06', -1)).toBe('2025-12-01');
  });

  it('is always drawn on six weeks from a Monday', () => {
    const weeks = weeksOf('2026-11-15');
    expect(weeks).toHaveLength(6);
    expect(weeks[0]).toBe('2026-10-26');
    expect(weeks[5]).toBe('2026-11-30');
  });
});

describe('zoomed', () => {
  it('goes through the scales', () => {
    expect(zoomed('month', 1)).toBe('quarter');
    expect(zoomed('month', -1)).toBe('week');
  });

  it('stops at both ends', () => {
    expect(zoomed('year', 1)).toBe('year');
    expect(zoomed('week', -1)).toBe('week');
  });
});

describe('moved', () => {
  it('moves by the scale shown', () => {
    expect(moved('week', '2026-11-06', 1)).toBe('2026-11-13');
    expect(moved('month', '2026-11-06', -1)).toBe('2026-10-01');
    expect(moved('quarter', '2026-11-06', 1)).toBe('2027-02-01');
    expect(moved('year', '2026-11-06', 1)).toBe('2027-11-01');
  });
});
