/** The digests of the program, in the order the tool lists them. Names are the same in every language. */
export const ALGORITHMS = [
  { id: 'md5', name: 'MD5', group: 'classic', hmac: true },
  { id: 'sha1', name: 'SHA-1', group: 'classic', hmac: true },
  { id: 'ripemd160', name: 'RIPEMD-160', group: 'classic', hmac: true },
  { id: 'sha224', name: 'SHA-224', group: 'sha2', hmac: true },
  { id: 'sha256', name: 'SHA-256', group: 'sha2', hmac: true },
  { id: 'sha384', name: 'SHA-384', group: 'sha2', hmac: true },
  { id: 'sha512', name: 'SHA-512', group: 'sha2', hmac: true },
  { id: 'sha512-256', name: 'SHA-512/256', group: 'sha2', hmac: true },
  { id: 'sha3-224', name: 'SHA3-224', group: 'sha3', hmac: true },
  { id: 'sha3-256', name: 'SHA3-256', group: 'sha3', hmac: true },
  { id: 'sha3-384', name: 'SHA3-384', group: 'sha3', hmac: true },
  { id: 'sha3-512', name: 'SHA3-512', group: 'sha3', hmac: true },
  { id: 'keccak256', name: 'Keccak-256', group: 'sha3', hmac: false },
  { id: 'shake128', name: 'SHAKE128', group: 'sha3', hmac: false },
  { id: 'shake256', name: 'SHAKE256', group: 'sha3', hmac: false },
  { id: 'blake2b', name: 'BLAKE2b', group: 'blake', hmac: false },
  { id: 'blake2s', name: 'BLAKE2s', group: 'blake', hmac: false },
  { id: 'blake3', name: 'BLAKE3', group: 'blake', hmac: false },
  { id: 'crc32', name: 'CRC32', group: 'checksum', hmac: false },
  { id: 'crc32c', name: 'CRC32C', group: 'checksum', hmac: false },
  { id: 'adler32', name: 'Adler-32', group: 'checksum', hmac: false },
  { id: 'xxh64', name: 'xxHash64', group: 'checksum', hmac: false },
] as const;

export type Algorithm = (typeof ALGORITHMS)[number];
export type AlgorithmId = Algorithm['id'];

/** Shown first for "hash <text>" in the search. */
export const USUAL: readonly AlgorithmId[] = ['md5', 'sha1', 'sha256', 'sha512', 'blake3'];

export const PASSWORD_KINDS = ['argon2id', 'bcrypt', 'scrypt', 'pbkdf2'] as const;
export type PasswordKind = (typeof PASSWORD_KINDS)[number];

export const PASSWORD_NAMES: Record<PasswordKind, string> = {
  argon2id: 'Argon2id',
  bcrypt: 'bcrypt',
  scrypt: 'scrypt',
  pbkdf2: 'PBKDF2',
};

export const algorithm = (id: string): Algorithm | undefined => ALGORITHMS.find((a) => a.id === id);
