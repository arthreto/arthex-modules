import { t } from './i18n';

const DEPARTMENTS = {
  Acting: 'acting',
  Directing: 'directing',
  Writing: 'writing',
  Production: 'production',
  Sound: 'sound',
  Camera: 'camera',
  Editing: 'editing',
  Art: 'art',
  'Costume & Make-Up': 'costume',
  Crew: 'crew',
  'Visual Effects': 'effects',
  Lighting: 'lighting',
  Creator: 'creator',
} as const;

const STATUSES = {
  'Returning Series': 'returning',
  Ended: 'ended',
  Canceled: 'canceled',
  'In Production': 'production',
  Planned: 'planned',
} as const;

const isKnown = <T extends object>(map: T, name: string): name is Extract<keyof T, string> =>
  Object.hasOwn(map, name);

/** A department as TMDB names it, in the language of the interface; an unknown one as it came. */
export const departmentOf = (name: string) =>
  isKnown(DEPARTMENTS, name) ? t(`departments.${DEPARTMENTS[name]}`) : name;

export const statusOf = (name: string) => (isKnown(STATUSES, name) ? t(`statuses.${STATUSES[name]}`) : name);

/** French ratings are ages; elsewhere they are names such as PG-13. */
export function certificationOf(rating: string): string {
  if (!rating) return '';
  if (/^\d+$/.test(rating)) return t('certification', { rating });
  return /^(U|TP|0|ALL|G)$/i.test(rating) ? t('allAges') : rating;
}
