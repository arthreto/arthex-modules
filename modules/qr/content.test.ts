import { describe, expect, it } from 'vitest';
import { contentOf, EMPTY_FIELDS, fileNameOf, startOf, type Fields } from './content';

const fields = (changed: Partial<Fields>): Fields => ({ ...EMPTY_FIELDS, ...changed });

describe('content of a code', () => {
  it('stays empty until the kind has what it needs', () => {
    for (const kind of ['link', 'text', 'wifi', 'contact', 'email', 'phone', 'sms', 'geo'] as const) {
      expect(contentOf(kind, EMPTY_FIELDS)).toBe('');
    }
  });

  it('turns a bare address into a web page and keeps other schemes', () => {
    expect(contentOf('link', fields({ link: 'github.com/arthreto' }))).toBe('https://github.com/arthreto');
    expect(contentOf('link', fields({ link: 'ftp://example.com' }))).toBe('ftp://example.com');
    expect(contentOf('link', fields({ link: 'just words' }))).toBe('just words');
  });

  it('escapes the separators of a Wi-Fi network and leaves out the password of an open one', () => {
    const network = fields({ ssid: 'Home;2', password: 'a:b', hidden: true });
    expect(contentOf('wifi', network)).toBe('WIFI:T:WPA;S:Home\\;2;P:a\\:b;H:true;;');
    expect(contentOf('wifi', { ...network, security: 'nopass', hidden: false })).toBe(
      'WIFI:T:nopass;S:Home\\;2;;',
    );
  });

  it('writes a vCard with only the lines filled in', () => {
    const card = contentOf(
      'contact',
      fields({ firstName: 'Camille', lastName: 'Martin', contactPhone: '+33 6 12 34 56 78' }),
    );
    expect(card.split('\n')).toEqual([
      'BEGIN:VCARD',
      'VERSION:3.0',
      'N:Martin;Camille',
      'FN:Camille Martin',
      'TEL:+33612345678',
      'END:VCARD',
    ]);
  });

  it('encodes the subject and body of an email', () => {
    expect(contentOf('email', fields({ emailTo: 'a@b.fr', emailSubject: 'Hi there', emailBody: '&' }))).toBe(
      'mailto:a@b.fr?subject=Hi%20there&body=%26',
    );
    expect(contentOf('email', fields({ emailTo: 'a@b.fr' }))).toBe('mailto:a@b.fr');
  });

  it('writes numbers without their spaces', () => {
    expect(contentOf('phone', fields({ phone: '+33 6 12.34' }))).toBe('tel:+3361234');
    expect(contentOf('sms', fields({ smsNumber: '06 12', smsMessage: 'On my way' }))).toBe(
      'SMSTO:0612:On my way',
    );
  });

  it('accepts a comma as the decimal separator and refuses a place off the globe', () => {
    expect(contentOf('geo', fields({ latitude: '48,8584', longitude: '2.2945' }))).toBe('geo:48.8584,2.2945');
    expect(contentOf('geo', fields({ latitude: '91', longitude: '0' }))).toBe('');
  });

  it('starts the Create tab on the kind of a searched text', () => {
    expect(startOf('https://github.com')).toEqual({ kind: 'link', fields: { link: 'https://github.com' } });
    expect(startOf('a@b.fr').kind).toBe('email');
    expect(startOf('+33 6 12 34 56 78').kind).toBe('phone');
    expect(startOf('Hello')).toEqual({ kind: 'text', fields: { text: 'Hello' } });
  });

  it('names the file after what the code holds', () => {
    expect(fileNameOf('link', EMPTY_FIELDS, 'https://www.github.com/x')).toBe('QR github.com');
    expect(fileNameOf('wifi', fields({ ssid: 'Home/2' }), '')).toBe('QR Home2');
    expect(fileNameOf('text', EMPTY_FIELDS, 'Hello')).toBe('QR code');
  });
});
