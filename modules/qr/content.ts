export const KINDS = ['link', 'text', 'wifi', 'contact', 'email', 'phone', 'sms', 'geo'] as const;
export type Kind = (typeof KINDS)[number];

export const SECURITIES = ['WPA', 'WEP', 'nopass'] as const;
export type Security = (typeof SECURITIES)[number];

/** Everything the Create tab asks for, whatever the kind of code. */
export interface Fields {
  link: string;
  text: string;
  ssid: string;
  password: string;
  security: Security;
  hidden: boolean;
  firstName: string;
  lastName: string;
  contactPhone: string;
  contactEmail: string;
  organization: string;
  emailTo: string;
  emailSubject: string;
  emailBody: string;
  phone: string;
  smsNumber: string;
  smsMessage: string;
  latitude: string;
  longitude: string;
}

export const EMPTY_FIELDS: Fields = {
  link: '',
  text: '',
  ssid: '',
  password: '',
  security: 'WPA',
  hidden: false,
  firstName: '',
  lastName: '',
  contactPhone: '',
  contactEmail: '',
  organization: '',
  emailTo: '',
  emailSubject: '',
  emailBody: '',
  phone: '',
  smsNumber: '',
  smsMessage: '',
  latitude: '',
  longitude: '',
};

/** The separators of the `WIFI:` and `MECARD:` formats, escaped inside a value. */
const escapeValue = (value: string) => value.replace(/([\\;,:"])/g, '\\$1');
const compactNumber = (value: string) => value.replace(/[\s.()-]/g, '');
const decimal = (value: string) => value.trim().replace(',', '.');

const BARE_ADDRESS = /^[\w-]+(\.[\w-]+)+(\/\S*)?$/;
const LINK = /^(https?:\/\/|www\.)\S+$/i;
const EMAIL = /^[^\s@]+@[^\s@]+\.[a-z]{2,}$/i;
const PHONE = /^\+?\d[\d\s.()-]{5,}$/;

/** A typed address without its scheme is a web page. */
function linkOf(value: string): string {
  const link = value.trim();
  return /^[a-z][a-z0-9+.-]*:/i.test(link) || !BARE_ADDRESS.test(link) ? link : `https://${link}`;
}

function wifiOf(f: Fields): string {
  if (!f.ssid.trim()) return '';
  const password = f.security === 'nopass' ? '' : `P:${escapeValue(f.password)};`;
  return `WIFI:T:${f.security};S:${escapeValue(f.ssid)};${password}${f.hidden ? 'H:true;' : ''};`;
}

function contactOf(f: Fields): string {
  const [first, last] = [f.firstName.trim(), f.lastName.trim()];
  if (![first, last, f.contactPhone.trim(), f.contactEmail.trim()].some(Boolean)) return '';
  const lines = ['BEGIN:VCARD', 'VERSION:3.0', `N:${last};${first}`, `FN:${`${first} ${last}`.trim()}`];
  if (f.organization.trim()) lines.push(`ORG:${f.organization.trim()}`);
  if (f.contactPhone.trim()) lines.push(`TEL:${compactNumber(f.contactPhone)}`);
  if (f.contactEmail.trim()) lines.push(`EMAIL:${f.contactEmail.trim()}`);
  lines.push('END:VCARD');
  return lines.join('\n');
}

function emailOf(f: Fields): string {
  if (!f.emailTo.trim()) return '';
  const query = [
    ['subject', f.emailSubject],
    ['body', f.emailBody],
  ]
    .filter(([, value]) => value?.trim())
    .map(([name, value]) => `${name}=${encodeURIComponent(value ?? '')}`);
  return `mailto:${f.emailTo.trim()}${query.length > 0 ? `?${query.join('&')}` : ''}`;
}

function geoOf(f: Fields): string {
  const [latitude, longitude] = [decimal(f.latitude), decimal(f.longitude)];
  const valid =
    latitude !== '' &&
    longitude !== '' &&
    Math.abs(Number(latitude)) <= 90 &&
    Math.abs(Number(longitude)) <= 180;
  return valid ? `geo:${latitude},${longitude}` : '';
}

/** What the code of a kind holds, in the formats phones read; empty while it is not filled in. */
export function contentOf(kind: Kind, f: Fields): string {
  switch (kind) {
    case 'link':
      return linkOf(f.link);
    case 'text':
      return f.text;
    case 'wifi':
      return wifiOf(f);
    case 'contact':
      return contactOf(f);
    case 'email':
      return emailOf(f);
    case 'phone':
      return f.phone.trim() ? `tel:${compactNumber(f.phone)}` : '';
    case 'sms':
      return f.smsNumber.trim() ? `SMSTO:${compactNumber(f.smsNumber)}:${f.smsMessage}` : '';
    case 'geo':
      return geoOf(f);
  }
}

/** Where the Create tab starts for a text typed in the search. */
export function startOf(text: string): { kind: Kind; fields: Partial<Fields> } {
  const value = text.trim();
  if (LINK.test(value) || (BARE_ADDRESS.test(value) && value.includes('/')))
    return { kind: 'link', fields: { link: value } };
  if (EMAIL.test(value)) return { kind: 'email', fields: { emailTo: value } };
  if (PHONE.test(value)) return { kind: 'phone', fields: { phone: value } };
  return { kind: 'text', fields: { text } };
}

/** The kind a text from the search would make. */
export const kindOfText = (text: string): Kind => startOf(text).kind;

/** A name for the saved file, from what the code holds. */
export function fileNameOf(kind: Kind, f: Fields, content: string): string {
  let base = '';
  if (kind === 'link') {
    try {
      base = new URL(content).hostname.replace(/^www\./, '');
    } catch {
      base = '';
    }
  } else if (kind === 'wifi') base = f.ssid.trim();
  else if (kind === 'contact') base = `${f.firstName.trim()} ${f.lastName.trim()}`.trim();
  return `QR ${base.replace(/[\\/:*?"<>|]/g, '').trim() || 'code'}`;
}
