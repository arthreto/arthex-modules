import { describe, expect, it } from 'vitest';
import {
  DEFAULT_CHOICES,
  cleanFormat,
  createRequest,
  familyOf,
  iconSize,
  nameProblem,
  sizeInBytes,
} from './choices';

describe('formats', () => {
  it('sorts formats into families, and an unknown one into an exact size', () => {
    expect(familyOf('png')).toBe('image');
    expect(familyOf('csv')).toBe('text');
    expect(familyOf('py')).toBe('code');
    expect(familyOf('xlsx')).toBe('document');
    expect(familyOf('mkv')).toBe('video');
    expect(familyOf('abc')).toBe('sized');
  });

  it('keeps only what a format can hold', () => {
    expect(cleanFormat(' .PNG ')).toBe('png');
    expect(cleanFormat('tar.gz')).toBe('targz');
    expect(cleanFormat('a'.repeat(20))).toHaveLength(12);
  });
});

describe('names', () => {
  it('refuses what Windows refuses', () => {
    expect(nameProblem('exemple')).toBeNull();
    expect(nameProblem('Réunion 日本 (2)')).toBeNull();
    expect(nameProblem('   ')).toBe('empty');
    expect(nameProblem('a/b')).toBe('characters');
    expect(nameProblem('quoi?')).toBe('characters');
    expect(nameProblem('CON')).toBe('reserved');
    expect(nameProblem('com1')).toBe('reserved');
    expect(nameProblem('fin.')).toBe('reserved');
  });
});

describe('sizes', () => {
  it('reads the size typed, between 1 KB and 10 GB', () => {
    expect(sizeInBytes('250', 'mb')).toBe(250 * 1024 ** 2);
    expect(sizeInBytes('1,5', 'gb')).toBe(1.5 * 1024 ** 3);
    expect(sizeInBytes('0', 'kb')).toBeNull();
    expect(sizeInBytes('11', 'gb')).toBeNull();
    expect(sizeInBytes('abc', 'mb')).toBeNull();
  });

  it('fits a picture in an icon', () => {
    expect(iconSize(1920, 1080)).toEqual([256, 144]);
    expect(iconSize(128, 64)).toEqual([128, 64]);
  });
});

describe('requests', () => {
  it('sends only the options of the family', () => {
    const choices = { ...DEFAULT_CHOICES, rows: 1000, size: [1920, 1080] as [number, number] };
    expect(createRequest('C:\\A', 'a', 'png', choices)).toEqual({
      folder: 'C:\\A',
      name: 'a',
      format: 'png',
      picture: 'landscape',
      size: [1920, 1080],
    });
    expect(createRequest('C:\\A', 'a', 'csv', choices)).toMatchObject({ rows: 1000 });
    expect(createRequest('C:\\A', 'a', 'py', choices)).not.toHaveProperty('rows');
    expect(createRequest('C:\\A', 'a', 'mp4', choices)).toMatchObject({ seconds: 30, height: 720 });
    expect(createRequest('C:\\A', 'a', 'iso', choices)).toMatchObject({
      bytes: 100 * 1024 ** 2,
      random: false,
    });
  });
});
