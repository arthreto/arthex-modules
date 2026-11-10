/** The voices of the model in its order: five women, five men, then the deep voice of the module. */
export type VoiceKind = 'woman' | 'man' | 'deep';

export interface Voice {
  id: number;
  kind: VoiceKind;
  /** Its number among the voices of its kind, from 1. */
  number: number;
}

export const VOICES: readonly Voice[] = Array.from({ length: 11 }, (_, id) =>
  id < 5
    ? { id, kind: 'woman', number: id + 1 }
    : id < 10
      ? { id, kind: 'man', number: id - 4 }
      : { id, kind: 'deep', number: 1 },
);

export const SPEEDS = [0.8, 1, 1.2, 1.5] as const;
export type Speed = (typeof SPEEDS)[number];

export const LANGUAGES = ['fr', 'en'] as const;
export type Language = (typeof LANGUAGES)[number];

export const FORMATS = ['wav', 'mp3'] as const;
export type Format = (typeof FORMATS)[number];

/** The language of the interface when the model speaks it, English otherwise. */
export const languageOf = (locale: string): Language => (locale.toLowerCase().startsWith('fr') ? 'fr' : 'en');

/** `0,8 ×` in French, `0.8 ×` in English. */
export const speedLabel = (speed: number, locale: string): string =>
  `${new Intl.NumberFormat(locale, { maximumFractionDigits: 1 }).format(speed)} ×`;

/** `1:05` for 65 seconds. */
export function clock(seconds: number): string {
  const whole = Math.max(0, Math.floor(seconds));
  return `${Math.floor(whole / 60)}:${String(whole % 60).padStart(2, '0')}`;
}
