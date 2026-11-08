import type { Fields, Kind, Security } from './content';

export type ReadKind = Kind | 'event' | 'otp';

export type FactLabel =
  | 'network'
  | 'security'
  | 'password'
  | 'hidden'
  | 'name'
  | 'phone'
  | 'email'
  | 'organization'
  | 'website'
  | 'address'
  | 'title'
  | 'start'
  | 'end'
  | 'place'
  | 'service'
  | 'account'
  | 'type'
  | 'digits'
  | 'secret'
  | 'to'
  | 'subject'
  | 'message'
  | 'number'
  | 'latitude'
  | 'longitude'
  | 'domain'
  | 'connection'
  | 'shortened'
  | 'misleading'
  | 'ipAddress'
  | 'tracking'
  | 'length';

export type FactMessage =
  | 'open'
  | 'wep'
  | 'wpa'
  | 'yes'
  | 'no'
  | 'secure'
  | 'insecure'
  | 'hiddenTarget'
  | 'foreignLetters'
  | 'ipInstead'
  | 'userInfo'
  | 'characters'
  | 'totp'
  | 'hotp'
  | 'keepSecret';

export type FactValue =
  | { text: string }
  | { message: FactMessage; values?: Record<string, string | number> }
  /** `YYYYMMDD` with `THHMM` when it has a time, as in calendar files. */
  | { day: string };

export interface Fact {
  label: FactLabel;
  value: FactValue;
  tone?: 'ok' | 'warn';
  secret?: boolean;
  mono?: boolean;
}

export type ActionId =
  | 'copy'
  | 'copyPassword'
  | 'connect'
  | 'addContact'
  | 'addEvent'
  | 'copyAddress'
  | 'writeEmail'
  | 'call'
  | 'copyNumber'
  | 'copyMessage'
  | 'copyCoordinates'
  | 'openMap'
  | 'openLink'
  | 'copyClean';

export interface Action {
  id: ActionId;
  /** What is copied or opened. */
  value: string;
  primary?: boolean;
}

export interface WifiNetwork {
  ssid: string;
  password: string;
  security: Security;
  hidden: boolean;
}

export interface Analysis {
  kind: ReadKind;
  facts: Fact[];
  actions: Action[];
  /** What the Create tab starts from to change the code. */
  edit: { kind: Kind; fields: Partial<Fields> } | null;
  network?: WifiNetwork;
}

const SHORTENERS = new Set([
  'bit.ly',
  'tinyurl.com',
  't.co',
  'goo.gl',
  'ow.ly',
  'is.gd',
  'buff.ly',
  'cutt.ly',
  'rebrand.ly',
  'shorturl.at',
  'tiny.cc',
  'lnkd.in',
  'qrco.de',
]);
const TRACKERS = /^(utm_[a-z]+|fbclid|gclid|dclid|msclkid|mc_eid|mc_cid|igshid|yclid|_hsenc|_hsmi)$/i;

const text = (value: string): FactValue => ({ text: value });
const message = (id: FactMessage, values?: Record<string, string | number>): FactValue => ({
  message: id,
  ...(values && { values }),
});

/** Splits `K:V;K:V;;` on the separators that are not escaped. */
export function pairsOf(body: string): Record<string, string> {
  const pairs: Record<string, string> = {};
  let key = '';
  let current = '';
  let inValue = false;
  for (let at = 0; at < body.length; at += 1) {
    const char = body.charAt(at);
    if (char === '\\' && at + 1 < body.length) {
      current += body.charAt(at + 1);
      at += 1;
    } else if (char === ':' && !inValue) {
      key = current.toUpperCase();
      current = '';
      inValue = true;
    } else if (char === ';') {
      if (key) pairs[key] = current;
      key = '';
      current = '';
      inValue = false;
    } else current += char;
  }
  if (key) pairs[key] = current;
  return pairs;
}

/** The first value of each property of a vCard or a calendar file. */
function propertiesOf(content: string): Record<string, string> {
  const properties: Record<string, string> = {};
  for (const line of content.split(/\r?\n/)) {
    const at = line.indexOf(':');
    if (at < 0) continue;
    const name = (line.slice(0, at).split(';')[0] ?? '').toUpperCase();
    if (!(name in properties)) properties[name] = line.slice(at + 1).trim();
  }
  return properties;
}

function wifi(content: string): Analysis {
  const pairs = pairsOf(content.slice('WIFI:'.length));
  const type = (pairs['T'] ?? '').toUpperCase();
  const security: Security = type === 'WEP' ? 'WEP' : type === '' || type === 'NOPASS' ? 'nopass' : 'WPA';
  const network: WifiNetwork = {
    ssid: pairs['S'] ?? '',
    password: pairs['P'] ?? '',
    security,
    hidden: pairs['H']?.toLowerCase() === 'true',
  };
  const facts: Fact[] = [
    { label: 'network', value: text(network.ssid) },
    security === 'nopass'
      ? { label: 'security', value: message('open'), tone: 'warn' }
      : security === 'WEP'
        ? { label: 'security', value: message('wep'), tone: 'warn' }
        : { label: 'security', value: message('wpa'), tone: 'ok' },
  ];
  if (network.password) facts.push({ label: 'password', value: text(network.password), secret: true });
  facts.push({ label: 'hidden', value: message(network.hidden ? 'yes' : 'no') });
  const actions: Action[] = network.password ? [{ id: 'copyPassword', value: network.password }] : [];
  actions.push({ id: 'connect', value: network.ssid, primary: true });
  return {
    kind: 'wifi',
    facts,
    actions,
    network,
    edit: {
      kind: 'wifi',
      fields: { ssid: network.ssid, password: network.password, security, hidden: network.hidden },
    },
  };
}

function contact(content: string): Analysis {
  const card = /^MECARD:/i.test(content) ? pairsOf(content.slice('MECARD:'.length)) : propertiesOf(content);
  const [last = '', first = ''] = (card['N'] ?? '').split(/[;,]/);
  const name = card['FN'] || `${first} ${last}`.trim();
  const facts: Fact[] = [{ label: 'name', value: text(name) }];
  const optional: [FactLabel, string | undefined][] = [
    ['phone', card['TEL']],
    ['email', card['EMAIL']],
    ['organization', card['ORG']],
    ['website', card['URL']],
    ['address', card['ADR']?.split(';').filter(Boolean).join(', ')],
  ];
  for (const [label, value] of optional) if (value) facts.push({ label, value: text(value) });
  return {
    kind: 'contact',
    facts,
    actions: [
      { id: 'copy', value: content },
      { id: 'addContact', value: content, primary: true },
    ],
    edit: {
      kind: 'contact',
      fields: {
        firstName: first,
        lastName: last,
        contactPhone: card['TEL'] ?? '',
        contactEmail: card['EMAIL'] ?? '',
        organization: card['ORG'] ?? '',
      },
    },
  };
}

function event(content: string): Analysis {
  const properties = propertiesOf(content);
  const facts: Fact[] = [{ label: 'title', value: text(properties['SUMMARY'] ?? '') }];
  if (properties['DTSTART']) facts.push({ label: 'start', value: { day: properties['DTSTART'] } });
  if (properties['DTEND']) facts.push({ label: 'end', value: { day: properties['DTEND'] } });
  if (properties['LOCATION']) facts.push({ label: 'place', value: text(properties['LOCATION']) });
  return {
    kind: 'event',
    facts,
    actions: [
      { id: 'copy', value: content },
      { id: 'addEvent', value: content, primary: true },
    ],
    edit: null,
  };
}

function oneTimePassword(content: string): Analysis {
  let url: URL | null = null;
  try {
    url = new URL(content);
  } catch {
    url = null;
  }
  const label = url ? decodeURIComponent(url.pathname.replace(/^\/+/, '')) : '';
  const service = url?.searchParams.get('issuer') || (label.includes(':') ? label.split(':')[0] : '') || '';
  const facts: Fact[] = [];
  if (service) facts.push({ label: 'service', value: text(service) });
  facts.push({ label: 'account', value: text(label.split(':').pop() ?? '') });
  facts.push({
    label: 'type',
    value:
      url?.host.toLowerCase() === 'hotp'
        ? message('hotp')
        : message('totp', { seconds: url?.searchParams.get('period') ?? '30' }),
  });
  facts.push({ label: 'digits', value: text(url?.searchParams.get('digits') ?? '6') });
  facts.push({ label: 'secret', value: message('keepSecret'), tone: 'warn' });
  return { kind: 'otp', facts, actions: [{ id: 'copy', value: content, primary: true }], edit: null };
}

function email(content: string): Analysis {
  let [to, subject, body] = ['', '', ''];
  if (/^MATMSG:/i.test(content)) {
    const pairs = pairsOf(content.slice('MATMSG:'.length));
    [to, subject, body] = [pairs['TO'] ?? '', pairs['SUB'] ?? '', pairs['BODY'] ?? ''];
  } else {
    const [address = '', query = ''] = content.slice('mailto:'.length).split('?');
    const params = new URLSearchParams(query);
    to = decodeURIComponent(address);
    subject = params.get('subject') ?? '';
    body = params.get('body') ?? '';
  }
  const facts: Fact[] = [{ label: 'to', value: text(to) }];
  if (subject) facts.push({ label: 'subject', value: text(subject) });
  if (body) facts.push({ label: 'message', value: text(body) });
  const query = [
    ['subject', subject],
    ['body', body],
  ]
    .filter(([, part]) => part)
    .map(([name, part]) => `${name}=${encodeURIComponent(part ?? '')}`);
  const mailto = /^mailto:/i.test(content)
    ? content
    : `mailto:${to}${query.length > 0 ? `?${query.join('&')}` : ''}`;
  return {
    kind: 'email',
    facts,
    actions: [
      { id: 'copyAddress', value: to },
      { id: 'writeEmail', value: mailto, primary: true },
    ],
    edit: { kind: 'email', fields: { emailTo: to, emailSubject: subject, emailBody: body } },
  };
}

function phone(content: string): Analysis {
  const number = content.slice('tel:'.length);
  return {
    kind: 'phone',
    facts: [{ label: 'number', value: text(number) }],
    actions: [
      { id: 'call', value: `tel:${number}` },
      { id: 'copyNumber', value: number, primary: true },
    ],
    edit: { kind: 'phone', fields: { phone: number } },
  };
}

function sms(content: string): Analysis {
  const rest = content.replace(/^(SMSTO|SMS):/i, '');
  const at = rest.indexOf(':');
  const number = at < 0 ? rest : rest.slice(0, at);
  const said = at < 0 ? '' : rest.slice(at + 1);
  const facts: Fact[] = [{ label: 'number', value: text(number) }];
  if (said) facts.push({ label: 'message', value: text(said) });
  return {
    kind: 'sms',
    facts,
    actions: said
      ? [
          { id: 'copyNumber', value: number },
          { id: 'copyMessage', value: said, primary: true },
        ]
      : [{ id: 'copyNumber', value: number, primary: true }],
    edit: { kind: 'sms', fields: { smsNumber: number, smsMessage: said } },
  };
}

function geo(latitude: string, longitude: string): Analysis {
  const coordinates = `${latitude}, ${longitude}`;
  return {
    kind: 'geo',
    facts: [
      { label: 'latitude', value: text(latitude) },
      { label: 'longitude', value: text(longitude) },
    ],
    actions: [
      { id: 'copyCoordinates', value: coordinates },
      {
        id: 'openMap',
        value: `https://www.openstreetmap.org/?mlat=${latitude}&mlon=${longitude}#map=16/${latitude}/${longitude}`,
        primary: true,
      },
    ],
    edit: { kind: 'geo', fields: { latitude, longitude } },
  };
}

function link(content: string, url: URL): Analysis {
  const host = url.hostname.toLowerCase();
  const domain = host.replace(/^www\./, '');
  const facts: Fact[] = [{ label: 'domain', value: text(domain), mono: true }];
  facts.push(
    url.protocol === 'https:'
      ? { label: 'connection', value: message('secure'), tone: 'ok' }
      : { label: 'connection', value: message('insecure'), tone: 'warn' },
  );
  if (SHORTENERS.has(domain))
    facts.push({ label: 'shortened', value: message('hiddenTarget'), tone: 'warn' });
  if (/(^|\.)xn--/.test(host))
    facts.push({ label: 'misleading', value: message('foreignLetters'), tone: 'warn' });
  if (/^\d{1,3}(\.\d{1,3}){3}$/.test(host) || host.startsWith('[')) {
    facts.push({ label: 'ipAddress', value: message('ipInstead'), tone: 'warn' });
  }
  if (url.username) {
    facts.push({
      label: 'misleading',
      value: message('userInfo', { host: domain, name: decodeURIComponent(url.username) }),
      tone: 'warn',
    });
  }
  const tracking = [...url.searchParams.keys()].filter((name) => TRACKERS.test(name));
  const actions: Action[] = [{ id: 'copy', value: content }];
  if (tracking.length > 0) {
    facts.push({ label: 'tracking', value: text([...new Set(tracking)].join(', ')), mono: true });
    actions.push({ id: 'copyClean', value: withoutTracking(url) });
  }
  actions.push({ id: 'openLink', value: url.href, primary: true });
  return { kind: 'link', facts, actions, edit: { kind: 'link', fields: { link: content } } };
}

/** The address without the parameters that only tell a site where the visit came from. */
export function withoutTracking(url: URL): string {
  const clean = new URL(url.href);
  for (const name of [...clean.searchParams.keys()]) if (TRACKERS.test(name)) clean.searchParams.delete(name);
  return clean.href.replace(/\?(?=#|$)/, '');
}

function plain(content: string): Analysis {
  return {
    kind: 'text',
    facts: [{ label: 'length', value: message('characters', { count: [...content].length }) }],
    actions: [{ id: 'copy', value: content, primary: true }],
    edit: { kind: 'text', fields: { text: content } },
  };
}

/** What a read code holds, what is worth knowing about it and what can be done with it. */
export function analyze(content: string): Analysis {
  const value = content.trim();
  if (/^WIFI:/i.test(value)) return wifi(value);
  if (/^(BEGIN:VCARD|MECARD:)/i.test(value)) return contact(value);
  if (/^BEGIN:(VEVENT|VCALENDAR)/i.test(value)) return event(value);
  if (/^otpauth:\/\//i.test(value)) return oneTimePassword(value);
  if (/^(mailto:|MATMSG:)/i.test(value)) return email(value);
  if (/^tel:/i.test(value)) return phone(value);
  if (/^(SMSTO|SMS):/i.test(value)) return sms(value);
  const place = /^geo:(-?\d+(?:\.\d+)?),(-?\d+(?:\.\d+)?)/i.exec(value);
  if (place?.[1] && place[2]) return geo(place[1], place[2]);
  if (/^(https?:\/\/|www\.)\S+$/i.test(value)) {
    try {
      return link(value, new URL(/^www\./i.test(value) ? `http://${value}` : value));
    } catch {
      return plain(content);
    }
  }
  return plain(content);
}
