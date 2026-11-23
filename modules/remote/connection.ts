/** A saved connection, as the program keeps it. Its secrets never come back to the interface. */

export const PROTOCOLS = ['sftp', 'ftp', 'ftps'] as const;
export type Protocol = (typeof PROTOCOLS)[number];
export type Auth = 'password' | 'key' | 'anonymous';

export interface Connection {
  id: string;
  name: string;
  protocol: Protocol;
  host: string;
  port: number;
  user: string;
  auth: Auth;
  /** The file name of the picked key, shown in the form. */
  keyName: string;
  /** The folder of the server shown as the drive. */
  folder: string;
  /** `Z`, or empty for no letter. */
  letter: string;
  autoConnect: boolean;
}

/** Which secrets the program holds for a connection. */
export interface Held {
  password: boolean;
  passphrase: boolean;
  key: boolean;
}

export type Status =
  | { state: 'off' }
  | { state: 'busy' }
  | { state: 'on'; place: string }
  | { state: 'trust'; algorithm: string; fingerprint: string }
  | { state: 'error'; reason: string; detail: string };

/** From the end of the alphabet, as network drives are usually given. */
export const LETTERS = 'ZYXWVUTSRQPONMLKJIHGFED'.split('');

export const defaultPort = (protocol: Protocol) => (protocol === 'sftp' ? 22 : 21);

export const authsOf = (protocol: Protocol): readonly Auth[] =>
  protocol === 'sftp' ? ['password', 'key'] : ['password', 'anonymous'];

export const newId = () => `${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`;

export function freeLetter(connections: readonly Connection[], except?: string): string {
  const used = new Set(connections.filter((other) => other.id !== except).map((other) => other.letter));
  return LETTERS.find((letter) => !used.has(letter)) ?? '';
}

export function newConnection(id: string, name: string, protocol: Protocol, others: readonly Connection[]): Connection {
  return {
    id,
    name,
    protocol,
    host: '',
    port: defaultPort(protocol),
    user: '',
    auth: 'password',
    keyName: '',
    folder: '/',
    letter: freeLetter(others),
    autoConnect: false,
  };
}

/** What changes with the protocol: a port left at its default follows it, and so does the login. */
export function withProtocol(connection: Connection, protocol: Protocol): Partial<Connection> {
  const port = connection.port === defaultPort(connection.protocol) ? defaultPort(protocol) : connection.port;
  const auth = authsOf(protocol).includes(connection.auth) ? connection.auth : 'password';
  return { protocol, port, auth };
}

/** A port typed by hand, or `null` while it is not one. */
export function parsePort(text: string): number | null {
  if (!/^\d{1,5}$/.test(text.trim())) return null;
  const port = Number(text);
  return port >= 1 && port <= 65535 ? port : null;
}

export function canConnect(connection: Connection, held: Held | undefined): boolean {
  if (connection.host.trim() === '') return false;
  if (connection.auth === 'anonymous') return true;
  if (connection.user.trim() === '') return false;
  return connection.auth !== 'key' || held?.key === true;
}
