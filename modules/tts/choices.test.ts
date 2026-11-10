import { describe, expect, it } from 'vitest';
import { VOICES, clock, languageOf, speedLabel } from './choices';

describe('voices', () => {
  it('follows the order of the model, the deep voice last', () => {
    expect(VOICES).toHaveLength(11);
    expect(VOICES[0]).toEqual({ id: 0, kind: 'woman', number: 1 });
    expect(VOICES[5]).toEqual({ id: 5, kind: 'man', number: 1 });
    expect(VOICES[9]).toEqual({ id: 9, kind: 'man', number: 5 });
    expect(VOICES[10]).toEqual({ id: 10, kind: 'deep', number: 1 });
  });
});

describe('labels', () => {
  it('writes speeds the way each language does', () => {
    expect(speedLabel(0.8, 'fr')).toBe('0,8 ×');
    expect(speedLabel(1, 'en')).toBe('1 ×');
    expect(speedLabel(1.5, 'en')).toBe('1.5 ×');
  });

  it('shows a length as minutes and seconds', () => {
    expect(clock(0)).toBe('0:00');
    expect(clock(9.7)).toBe('0:09');
    expect(clock(65)).toBe('1:05');
  });

  it('speaks the language of the interface when it can', () => {
    expect(languageOf('fr-FR')).toBe('fr');
    expect(languageOf('en')).toBe('en');
    expect(languageOf('de')).toBe('en');
  });
});
