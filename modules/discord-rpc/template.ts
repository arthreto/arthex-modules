export const KINDS = ['playing', 'listening', 'watching', 'competing'] as const;
export const SHOWS = ['name', 'details', 'state'] as const;
export const TIMES = ['none', 'elapsed', 'left', 'dates'] as const;
export const MAX_BUTTONS = 2;
export const MAX_TEXT_LENGTH = 128;
export const MAX_LABEL_LENGTH = 32;
export const MAX_URL_LENGTH = 512;

export type Kind = (typeof KINDS)[number];
export type Show = (typeof SHOWS)[number];
export type TimeMode = (typeof TIMES)[number];

export interface TemplateButton {
  label: string;
  url: string;
}

export interface Template {
  id: string;
  name: string;
  appId: string;
  /** The name shown instead of the application's. */
  title: string;
  kind: Kind;
  show: Show;
  details: string;
  detailsUrl: string;
  state: string;
  stateUrl: string;
  /** The name of an image imported on the application, or an address. */
  large: string;
  largeText: string;
  largeUrl: string;
  small: string;
  smallText: string;
  smallUrl: string;
  time: TimeMode;
  minutes: number | null;
  /** Milliseconds. */
  from: number | null;
  /** Milliseconds. */
  to: number | null;
  size: number | null;
  max: number | null;
  buttons: TemplateButton[];
}

export type FieldError = 'digits' | 'address' | 'short' | 'party';

export function newTemplate(id: string, name: string, kind: Kind): Template {
  return {
    id,
    name,
    appId: '',
    title: '',
    kind,
    show: 'name',
    details: '',
    detailsUrl: '',
    state: '',
    stateUrl: '',
    large: '',
    largeText: '',
    largeUrl: '',
    small: '',
    smallText: '',
    smallUrl: '',
    time: 'none',
    minutes: null,
    from: null,
    to: null,
    size: null,
    max: null,
    buttons: [],
  };
}

export const isAppId = (value: string): boolean => /^\d+$/.test(value);

export const canTurnOn = (template: Template): boolean => isAppId(template.appId);

export const appIdError = (value: string): FieldError | null => (value && !isAppId(value) ? 'digits' : null);

export const addressError = (value: string): FieldError | null =>
  value.trim() && !/^https?:\/\//.test(value.trim()) ? 'address' : null;

/** Discord refuses a text of a single character. */
export const textError = (value: string): FieldError | null => (value.trim().length === 1 ? 'short' : null);

export const partyError = (size: number | null, max: number | null): FieldError | null =>
  size !== null && max !== null && size > max ? 'party' : null;

/** What a number field holds: nothing, or a whole number. */
export function parseCount(value: string): number | null {
  if (!/^\d+$/.test(value.trim())) return null;
  const count = Number(value);
  return Number.isSafeInteger(count) ? count : null;
}

const twoDigits = (value: number) => String(value).padStart(2, '0');

/** The value of a `datetime-local` field, in the local time. */
export function toLocalInput(time: number | null): string {
  if (time === null) return '';
  const date = new Date(time);
  const day = `${date.getFullYear()}-${twoDigits(date.getMonth() + 1)}-${twoDigits(date.getDate())}`;
  return `${day}T${twoDigits(date.getHours())}:${twoDigits(date.getMinutes())}`;
}

export function fromLocalInput(value: string): number | null {
  const time = new Date(value).getTime();
  return Number.isNaN(time) ? null : time;
}

export function clock(totalSeconds: number): string {
  const seconds = Math.max(0, Math.floor(totalSeconds));
  const hours = Math.floor(seconds / 3600);
  const minutes = twoDigits(Math.floor((seconds % 3600) / 60));
  return `${hours > 0 ? `${hours}:` : ''}${minutes}:${twoDigits(seconds % 60)}`;
}

/** How far `now` is between two dates, from 0 to 1, or `null` without a valid span. */
export function progress(from: number | null, to: number | null, now: number): number | null {
  if (from === null || to === null || to <= from) return null;
  return Math.min(1, Math.max(0, (now - from) / (to - from)));
}
