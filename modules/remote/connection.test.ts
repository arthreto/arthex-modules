import { describe, expect, it } from 'vitest';
import { canConnect, freeLetter, newConnection, parsePort, withProtocol, type Connection } from './connection';

const sample = (changes: Partial<Connection> = {}): Connection => ({
  ...newConnection('a', 'VPS', 'sftp', []),
  ...changes,
});

describe('connections', () => {
  it('gives each new connection a letter nobody uses', () => {
    expect(freeLetter([])).toBe('Z');
    expect(freeLetter([sample({ letter: 'Z' }), sample({ id: 'b', letter: 'Y' })])).toBe('X');
    expect(freeLetter([sample({ letter: 'Z' })], 'a')).toBe('Z');
    expect(newConnection('b', 'NAS', 'ftp', [sample({ letter: 'Z' })])).toMatchObject({ port: 21, letter: 'Y' });
  });

  it('follows the protocol with a default port and a login it offers', () => {
    expect(withProtocol(sample(), 'ftp')).toEqual({ protocol: 'ftp', port: 21, auth: 'password' });
    expect(withProtocol(sample({ port: 2222 }), 'ftps').port).toBe(2222);
    expect(withProtocol(sample({ auth: 'key' }), 'ftp').auth).toBe('password');
    expect(withProtocol(sample({ protocol: 'ftp', port: 21, auth: 'anonymous' }), 'sftp')).toEqual({
      protocol: 'sftp',
      port: 22,
      auth: 'password',
    });
  });

  it('reads ports', () => {
    expect(parsePort('22')).toBe(22);
    expect(parsePort(' 2222 ')).toBe(2222);
    expect(parsePort('0')).toBeNull();
    expect(parsePort('70000')).toBeNull();
    expect(parsePort('2a')).toBeNull();
  });

  it('connects once it knows the server and who logs in', () => {
    expect(canConnect(sample(), undefined)).toBe(false);
    expect(canConnect(sample({ host: 'example.com', user: 'root' }), undefined)).toBe(true);
    expect(canConnect(sample({ host: 'example.com', user: 'root', auth: 'key' }), undefined)).toBe(false);
    const held = { password: false, passphrase: false, key: true };
    expect(canConnect(sample({ host: 'example.com', user: 'root', auth: 'key' }), held)).toBe(true);
    expect(canConnect(sample({ protocol: 'ftp', host: 'example.com', auth: 'anonymous' }), undefined)).toBe(true);
  });
});
