import { describe, expect, it } from 'vitest';
import { analyze, pairsOf, withoutTracking } from './analysis';

const labels = (content: string) => analyze(content).facts.map((fact) => fact.label);
const actions = (content: string) => analyze(content).actions.map((action) => action.id);

describe('analysis of a read code', () => {
  it('splits pairs on the separators that are not escaped', () => {
    expect(pairsOf('T:WPA;S:Home\\;2;P:a\\:b;;')).toEqual({ T: 'WPA', S: 'Home;2', P: 'a:b' });
  });

  it('reads a Wi-Fi network, hides its password and warns about weak security', () => {
    const network = analyze('WIFI:T:WPA;S:Home;P:secret;;');
    expect(network.kind).toBe('wifi');
    expect(network.network).toEqual({ ssid: 'Home', password: 'secret', security: 'WPA', hidden: false });
    expect(network.facts.find((fact) => fact.label === 'password')?.secret).toBe(true);
    expect(network.edit).toEqual({
      kind: 'wifi',
      fields: { ssid: 'Home', password: 'secret', security: 'WPA', hidden: false },
    });
    expect(analyze('WIFI:T:WEP;S:Old;P:x;;').facts[1]?.tone).toBe('warn');
    expect(analyze('WIFI:T:nopass;S:Cafe;;').facts[1]).toEqual({
      label: 'security',
      value: { message: 'open' },
      tone: 'warn',
    });
    expect(actions('WIFI:T:nopass;S:Cafe;;')).toEqual(['connect']);
  });

  it('warns about what makes a link risky', () => {
    expect(analyze('https://github.com').facts[1]?.tone).toBe('ok');
    expect(labels('http://bit.ly/3abc')).toEqual(['domain', 'connection', 'shortened']);
    expect(analyze('http://bit.ly/3abc').facts[1]?.tone).toBe('warn');
    expect(labels('https://xn--pypal-4ve.com')).toContain('misleading');
    expect(labels('http://192.168.1.10/admin')).toContain('ipAddress');
    expect(analyze('https://paypal.com@evil.com/').facts.at(-1)?.value).toEqual({
      message: 'userInfo',
      values: { host: 'evil.com', name: 'paypal.com' },
    });
  });

  it('lists tracking parameters and offers the link without them', () => {
    const link = 'https://example.com/page?utm_source=poster&id=4&fbclid=x#top';
    expect(analyze(link).facts.find((fact) => fact.label === 'tracking')?.value).toEqual({
      text: 'utm_source, fbclid',
    });
    expect(actions(link)).toEqual(['copy', 'copyClean', 'openLink']);
    expect(withoutTracking(new URL(link))).toBe('https://example.com/page?id=4#top');
    expect(withoutTracking(new URL('https://example.com/?utm_campaign=x'))).toBe('https://example.com/');
  });

  it('reads contacts from a vCard or a MeCard', () => {
    const card = analyze(
      'BEGIN:VCARD\nVERSION:3.0\nN:Martin;Camille\nFN:Camille Martin\nTEL;TYPE=CELL:+336\nEND:VCARD',
    );
    expect(card.kind).toBe('contact');
    expect(card.facts.map((fact) => fact.value)).toEqual([{ text: 'Camille Martin' }, { text: '+336' }]);
    expect(analyze('MECARD:N:Martin,Camille;TEL:+336;;').edit?.fields).toMatchObject({
      firstName: 'Camille',
      lastName: 'Martin',
    });
  });

  it('reads emails, numbers, messages and places', () => {
    expect(analyze('mailto:a@b.fr?subject=Hi%20there').facts.map((fact) => fact.value)).toEqual([
      { text: 'a@b.fr' },
      { text: 'Hi there' },
    ]);
    expect(analyze('MATMSG:TO:a@b.fr;SUB:Hi;BODY:Yo;;').actions[1]?.value).toBe(
      'mailto:a@b.fr?subject=Hi&body=Yo',
    );
    expect(analyze('tel:+3361234').edit).toEqual({ kind: 'phone', fields: { phone: '+3361234' } });
    expect(analyze('SMSTO:0612:On my way').edit).toEqual({
      kind: 'sms',
      fields: { smsNumber: '0612', smsMessage: 'On my way' },
    });
    expect(analyze('geo:48.8584,2.2945').actions[1]?.value).toContain('mlat=48.8584&mlon=2.2945');
  });

  it('warns that a two-factor code gives access to an account', () => {
    const code = analyze('otpauth://totp/GitHub:arthreto?secret=JBSWY3DP&issuer=GitHub');
    expect(code.kind).toBe('otp');
    expect(code.facts.map((fact) => fact.label)).toEqual(['service', 'account', 'type', 'digits', 'secret']);
    expect(code.edit).toBeNull();
  });

  it('reads an event and keeps anything else as text', () => {
    const event = analyze('BEGIN:VEVENT\nSUMMARY:Concert\nDTSTART:20261120T200000\nEND:VEVENT');
    expect(event.facts).toEqual([
      { label: 'title', value: { text: 'Concert' } },
      { label: 'start', value: { day: '20261120T200000' } },
    ]);
    expect(analyze('Héllo 👋').facts[0]?.value).toEqual({ message: 'characters', values: { count: 7 } });
  });
});
