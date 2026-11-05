import { describe, expect, it } from 'vitest';
import {
  addressError,
  appIdError,
  canTurnOn,
  clock,
  fromLocalInput,
  newTemplate,
  parseCount,
  partyError,
  progress,
  textError,
  toLocalInput,
} from './template';

describe('fields', () => {
  it('takes only digits as an application ID', () => {
    expect(appIdError('')).toBeNull();
    expect(appIdError('1234567890')).toBeNull();
    expect(appIdError('12a')).toBe('digits');
  });

  it('needs an application ID to turn a template on', () => {
    const template = newTemplate('a', 'A', 'playing');
    expect(canTurnOn(template)).toBe(false);
    expect(canTurnOn({ ...template, appId: 'abc' })).toBe(false);
    expect(canTurnOn({ ...template, appId: '123' })).toBe(true);
  });

  it('takes only web addresses', () => {
    expect(addressError('')).toBeNull();
    expect(addressError('https://example.com')).toBeNull();
    expect(addressError('http://example.com')).toBeNull();
    expect(addressError('file:///c:/a')).toBe('address');
    expect(addressError('example.com')).toBe('address');
  });

  it('refuses a text of one character', () => {
    expect(textError('')).toBeNull();
    expect(textError(' a ')).toBe('short');
    expect(textError('ab')).toBeNull();
  });

  it('refuses more people than the maximum', () => {
    expect(partyError(2, 4)).toBeNull();
    expect(partyError(5, 4)).toBe('party');
    expect(partyError(5, null)).toBeNull();
  });

  it('reads whole numbers only', () => {
    expect(parseCount('12')).toBe(12);
    expect(parseCount('')).toBeNull();
    expect(parseCount('1.5')).toBeNull();
    expect(parseCount('-1')).toBeNull();
    expect(parseCount('99999999999999999999')).toBeNull();
  });
});

describe('time', () => {
  it('goes to a date field and back', () => {
    const time = new Date(2026, 10, 5, 18, 30).getTime();
    expect(toLocalInput(time)).toBe('2026-11-05T18:30');
    expect(fromLocalInput('2026-11-05T18:30')).toBe(time);
    expect(toLocalInput(null)).toBe('');
    expect(fromLocalInput('')).toBeNull();
  });

  it('writes a duration as a clock', () => {
    expect(clock(0)).toBe('00:00');
    expect(clock(754)).toBe('12:34');
    expect(clock(3725)).toBe('1:02:05');
    expect(clock(-5)).toBe('00:00');
  });

  it('places now between two dates', () => {
    expect(progress(100, 200, 150)).toBe(0.5);
    expect(progress(100, 200, 50)).toBe(0);
    expect(progress(100, 200, 500)).toBe(1);
    expect(progress(200, 100, 150)).toBeNull();
    expect(progress(null, 200, 150)).toBeNull();
  });
});
