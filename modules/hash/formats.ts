export type Format = 'hex' | 'HEX' | 'base64';

const B64 = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/';

export const bytesOfHex = (hex: string): Uint8Array =>
  Uint8Array.from(hex.match(/../g) ?? [], (pair) => parseInt(pair, 16));

export const hexOf = (bytes: Uint8Array): string =>
  Array.from(bytes, (byte) => byte.toString(16).padStart(2, '0')).join('');

export function base64(bytes: Uint8Array, padded = true): string {
  let out = '';
  for (let i = 0; i < bytes.length; i += 3) {
    const n = ((bytes[i] ?? 0) << 16) | ((bytes[i + 1] ?? 0) << 8) | (bytes[i + 2] ?? 0);
    out += (B64[n >> 18] ?? '') + (B64[(n >> 12) & 63] ?? '');
    out += i + 1 < bytes.length ? (B64[(n >> 6) & 63] ?? '') : padded ? '=' : '';
    out += i + 2 < bytes.length ? (B64[n & 63] ?? '') : padded ? '=' : '';
  }
  return out;
}

/** A digest of the program, which gives it in lower-case hex, written in the chosen format. */
export function shown(hex: string, format: Format): string {
  if (format === 'HEX') return hex.toUpperCase();
  if (format === 'base64') return base64(bytesOfHex(hex));
  return hex;
}

/** Whether a pasted digest is this one, in hex of either case or in Base64, spaces aside. */
export function matches(hex: string, pasted: string): boolean {
  const wanted = pasted.replace(/\s+/g, '');
  return wanted !== '' && (wanted.toLowerCase() === hex || wanted === base64(bytesOfHex(hex)));
}

export type KeyFormat = 'hex' | 'base64' | 'base64url' | 'uuid';

/** A random key of the chosen length, or a version 4 UUID from its first 16 bytes. */
export function keyText(bytes: Uint8Array, bits: number, format: KeyFormat): string {
  if (format === 'uuid') {
    const b = bytes.slice(0, 16);
    b[6] = ((b[6] ?? 0) & 0x0f) | 0x40;
    b[8] = ((b[8] ?? 0) & 0x3f) | 0x80;
    const hex = hexOf(b);
    return [hex.slice(0, 8), hex.slice(8, 12), hex.slice(12, 16), hex.slice(16, 20), hex.slice(20)].join('-');
  }
  const key = bytes.slice(0, bits / 8);
  if (format === 'base64') return base64(key);
  if (format === 'base64url') return base64(key, false).replace(/\+/g, '-').replace(/\//g, '_');
  return hexOf(key);
}
