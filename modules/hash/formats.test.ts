import { describe, expect, it } from 'vitest';
import { base64, bytesOfHex, hexOf, keyText, matches, shown } from './formats';

const SHA256_BONJOUR = '2cb4b1431b84ec15d35ed83bb927e27e8967d75f4bcd9cc4b25c8d879ae23e18';

describe('formats', () => {
  it('writes a digest in hex, upper-case hex or Base64', () => {
    expect(shown(SHA256_BONJOUR, 'hex')).toBe(SHA256_BONJOUR);
    expect(shown(SHA256_BONJOUR, 'HEX')).toBe(SHA256_BONJOUR.toUpperCase());
    expect(shown(SHA256_BONJOUR, 'base64')).toBe('LLSxQxuE7BXTXtg7uSfifoln119LzZzEslyNh5riPhg=');
  });

  it('pads Base64 only when asked', () => {
    expect(base64(Uint8Array.from([1]))).toBe('AQ==');
    expect(base64(Uint8Array.from([1]), false)).toBe('AQ');
    expect(base64(Uint8Array.from([1, 2, 3]))).toBe('AQID');
  });

  it('goes from hex to bytes and back', () => {
    expect(hexOf(bytesOfHex(SHA256_BONJOUR))).toBe(SHA256_BONJOUR);
  });

  it('recognises a pasted digest in any of its forms', () => {
    expect(matches(SHA256_BONJOUR, SHA256_BONJOUR.toUpperCase())).toBe(true);
    expect(matches(SHA256_BONJOUR, ` ${SHA256_BONJOUR.slice(0, 10)} ${SHA256_BONJOUR.slice(10)} `)).toBe(
      true,
    );
    expect(matches(SHA256_BONJOUR, 'LLSxQxuE7BXTXtg7uSfifoln119LzZzEslyNh5riPhg=')).toBe(true);
    expect(matches(SHA256_BONJOUR, 'llsxqxue7bxtxtg7usfifoln119lzzzeslynh5riphg=')).toBe(false);
    expect(matches(SHA256_BONJOUR, '')).toBe(false);
  });

  it('writes keys of the chosen length and a valid UUID', () => {
    const bytes = Uint8Array.from({ length: 64 }, (_, i) => i * 7);
    expect(keyText(bytes, 128, 'hex')).toHaveLength(32);
    expect(keyText(bytes, 512, 'hex')).toHaveLength(128);
    expect(keyText(bytes, 256, 'base64url')).not.toMatch(/[+/=]/);
    expect(keyText(bytes, 256, 'uuid')).toMatch(
      /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/,
    );
  });
});
