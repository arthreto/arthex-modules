import { ALGORITHMS, type AlgorithmId, type PasswordKind } from './algorithms';

export type HashQuery =
  | { kind: 'one'; algorithm: AlgorithmId; text: string }
  | { kind: 'all'; text: string }
  | { kind: 'digest'; bits: number; hex: string }
  | { kind: 'password'; algorithm: PasswordKind; stored: string };

/** Other ways of writing a name, lower case, beside the name itself without its dash. */
const ALIASES: Record<string, AlgorithmId> = {
  ripemd: 'ripemd160',
  'sha512/256': 'sha512-256',
  sha3: 'sha3-256',
  keccak: 'keccak256',
  blake2: 'blake2b',
  crc: 'crc32',
  adler: 'adler32',
  xxhash: 'xxh64',
  xxhash64: 'xxh64',
};

const DIGEST_LENGTHS = new Set([8, 16, 32, 40, 56, 64, 96, 128]);

const PASSWORD_FORMATS: [RegExp, PasswordKind][] = [
  [/^\$argon2id\$/, 'argon2id'],
  [/^\$2[aby]\$\d\d\$/, 'bcrypt'],
  [/^\$scrypt\$/, 'scrypt'],
  [/^\$pbkdf2-sha256\$/, 'pbkdf2'],
];

export function algorithmNamed(word: string): AlgorithmId | undefined {
  const lower = word.toLowerCase();
  const known = ALGORITHMS.find(
    (a) =>
      a.id === lower || a.name.toLowerCase() === lower || a.name.toLowerCase().replace('-', '') === lower,
  );
  return known?.id ?? ALIASES[lower];
}

/** What the search bar asks of Hash, if anything. `allWords` ask for every usual digest. */
export function parseQuery(text: string, allWords: readonly string[]): HashQuery | null {
  const trimmed = text.trim();
  const asked = /^(\S+)\s+([\s\S]+)$/.exec(trimmed);
  if (asked?.[1] && asked[2]) {
    const word = asked[1].toLowerCase();
    if (allWords.includes(word)) return { kind: 'all', text: asked[2] };
    const algorithm = algorithmNamed(word);
    if (algorithm) return { kind: 'one', algorithm, text: asked[2] };
  }
  if (/^[0-9a-f]+$/i.test(trimmed) && DIGEST_LENGTHS.has(trimmed.length)) {
    return { kind: 'digest', bits: trimmed.length * 4, hex: trimmed.toLowerCase() };
  }
  const password = PASSWORD_FORMATS.find(([format]) => format.test(trimmed));
  return password ? { kind: 'password', algorithm: password[1], stored: trimmed } : null;
}

/** The digests whose length in bits is this one. */
export const algorithmsOfBits = (bits: number): AlgorithmId[] =>
  ALGORITHMS.filter((a) => DIGEST_BITS[a.id] === bits).map((a) => a.id);

const DIGEST_BITS: Record<AlgorithmId, number> = {
  md5: 128,
  sha1: 160,
  ripemd160: 160,
  sha224: 224,
  sha256: 256,
  sha384: 384,
  sha512: 512,
  'sha512-256': 256,
  'sha3-224': 224,
  'sha3-256': 256,
  'sha3-384': 384,
  'sha3-512': 512,
  keccak256: 256,
  shake128: 256,
  shake256: 512,
  blake2b: 512,
  blake2s: 256,
  blake3: 256,
  crc32: 32,
  crc32c: 32,
  adler32: 32,
  xxh64: 64,
};
