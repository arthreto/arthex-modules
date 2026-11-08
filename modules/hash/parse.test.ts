import { describe, expect, it } from 'vitest';
import { algorithmNamed, algorithmsOfBits, parseQuery } from './parse';

const ALL = ['hash', 'empreinte'];

describe('parseQuery', () => {
  it('reads an algorithm and the text after it, in any spelling', () => {
    expect(parseQuery('sha256 bonjour', ALL)).toEqual({ kind: 'one', algorithm: 'sha256', text: 'bonjour' });
    expect(parseQuery('SHA-256 bonjour tout le monde', ALL)).toEqual({
      kind: 'one',
      algorithm: 'sha256',
      text: 'bonjour tout le monde',
    });
    expect(parseQuery('sha3-512 x', ALL)).toMatchObject({ algorithm: 'sha3-512' });
    expect(parseQuery('keccak x', ALL)).toMatchObject({ algorithm: 'keccak256' });
    expect(parseQuery('xxhash x', ALL)).toMatchObject({ algorithm: 'xxh64' });
    expect(parseQuery('sha512/256 x', ALL)).toMatchObject({ algorithm: 'sha512-256' });
  });

  it('asks for every usual digest after one of its words', () => {
    expect(parseQuery('hash Arthex', ALL)).toEqual({ kind: 'all', text: 'Arthex' });
    expect(parseQuery('Empreinte Arthex', ALL)).toEqual({ kind: 'all', text: 'Arthex' });
  });

  it('recognises a pasted digest by its length', () => {
    expect(parseQuery('2CB4B1431B84EC15D35ED83BB927E27E8967D75F4BCD9CC4B25C8D879AE23E18', ALL)).toEqual({
      kind: 'digest',
      bits: 256,
      hex: '2cb4b1431b84ec15d35ed83bb927e27e8967d75f4bcd9cc4b25c8d879ae23e18',
    });
    expect(parseQuery('abc123', ALL)).toBeNull();
  });

  it('recognises a pasted password hash', () => {
    expect(parseQuery('$2b$12$nqXPij6wiTL4aG4.0eMbBO9PyMV/KID/K6pH7Buih5jTJ6ioN0O5m', ALL)).toMatchObject({
      kind: 'password',
      algorithm: 'bcrypt',
    });
    expect(parseQuery('$argon2id$v=19$m=19456,t=2,p=1$abc$def', ALL)).toMatchObject({
      algorithm: 'argon2id',
    });
  });

  it('leaves everything else to others', () => {
    expect(parseQuery('bonjour', ALL)).toBeNull();
    expect(parseQuery('rapport 2026', ALL)).toBeNull();
  });
});

describe('algorithms', () => {
  it('knows names with or without their dash', () => {
    expect(algorithmNamed('sha1')).toBe('sha1');
    expect(algorithmNamed('SHA-1')).toBe('sha1');
    expect(algorithmNamed('ripemd-160')).toBe('ripemd160');
    expect(algorithmNamed('word')).toBeUndefined();
  });

  it('lists the digests of a length', () => {
    expect(algorithmsOfBits(160)).toEqual(['sha1', 'ripemd160']);
    expect(algorithmsOfBits(32)).toEqual(['crc32', 'crc32c', 'adler32']);
  });
});
