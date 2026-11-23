import { moduleNative, toast } from '@arthex/sdk';
import {
  canConnect,
  newConnection,
  newId,
  freeLetter,
  type Connection,
  type Held,
  type Protocol,
  type Status,
} from './connection';
import { t } from './i18n';
import { ID } from './identity';

interface ProgramState {
  connections: Connection[];
  secrets: Record<string, Held>;
  statuses: Record<string, Status>;
}

const SAVE_DELAY_MS = 300;
const SECRET_DELAY_MS = 400;
const OFF: Status = { state: 'off' };
const NONE: Held = { password: false, passphrase: false, key: false };

const native = moduleNative(ID);

/** `Z:` for a drive with a letter, else nothing: it is then reached by its folder. */
export const placeOf = (connection: Connection) => (connection.letter ? `${connection.letter}:` : '');

class Remote {
  connections = $state<Connection[]>([]);
  held = $state<Record<string, Held>>({});
  statuses = $state<Record<string, Status>>({});
  selected = $state<string | null>(null);
  removing = $state<Connection | null>(null);
  /** A file that was not a key, shown under the key field until another is picked. */
  keyError = $state(false);
  current = $derived(this.connections.find((connection) => connection.id === this.selected) ?? null);
  /** The connection whose server waits for the user's trust. */
  trusting = $derived(this.connections.find((connection) => this.statusOf(connection.id).state === 'trust') ?? null);

  /** Nothing is saved before the program has answered, or an empty list would replace its own. */
  #loaded = false;
  #loading: Promise<void> | null = null;
  #pendingSave: ReturnType<typeof setTimeout> | null = null;
  #pendingSecrets = new Map<string, { timer: ReturnType<typeof setTimeout>; send: () => Promise<void> }>();
  /** Connections to open in Explorer once connected, as asked from the search. */
  #openWhenOn = new Set<string>();

  /** Each opening asks again: events wait while the page is suspended. */
  load(): Promise<void> {
    this.#loading = native
      .call<ProgramState>('state')
      .then((state) => {
        this.statuses = state.statuses;
        if (this.#loaded) return;
        this.connections = state.connections;
        this.held = state.secrets;
        this.selected ??= state.connections[0]?.id ?? null;
        this.#loaded = true;
        native.on('status', (payload) => {
          const { id, status } = payload as { id: string; status: Status };
          this.#changed(id, status);
        });
      })
      .catch(() => toast(t('loadFailed')));
    return this.#loading;
  }

  /** For the search: the list once, without asking again at each keystroke. */
  ready(): Promise<void> {
    return this.#loaded ? Promise.resolve() : (this.#loading ?? this.load());
  }

  statusOf(id: string): Status {
    return this.statuses[id] ?? OFF;
  }

  heldOf(id: string): Held {
    return this.held[id] ?? NONE;
  }

  canConnect(connection: Connection): boolean {
    return canConnect(connection, this.held[connection.id]);
  }

  select(id: string): void {
    this.selected = id;
    this.keyError = false;
  }

  create(protocol: Protocol): void {
    const connection = newConnection(newId(), t('newName'), protocol, this.connections);
    this.connections.push(connection);
    this.select(connection.id);
    this.saveNow();
  }

  duplicate(connection: Connection): void {
    const copy: Connection = {
      ...$state.snapshot(connection),
      id: newId(),
      name: t('copyName', { name: connection.name }),
      letter: freeLetter(this.connections),
      keyName: '',
      auth: connection.auth === 'key' ? 'password' : connection.auth,
      autoConnect: false,
    };
    this.connections.push(copy);
    this.select(copy.id);
    this.saveNow();
  }

  remove(connection: Connection): void {
    this.connections = this.connections.filter((other) => other.id !== connection.id);
    if (this.selected === connection.id) this.selected = this.connections[0]?.id ?? null;
    delete this.statuses[connection.id];
    this.removing = null;
    this.saveNow();
    toast(t('removed', { name: connection.name }));
  }

  update(changes: Partial<Connection>): void {
    if (!this.current) return;
    Object.assign(this.current, changes);
    this.#saveSoon();
  }

  /** Sent as typed, a little later: the program encrypts it and never gives it back. */
  setSecret(field: 'password' | 'passphrase', value: string): void {
    const connection = this.current;
    if (!connection) return;
    const id = connection.id;
    this.held[id] = { ...this.heldOf(id), [field]: value !== '' };
    const key = `${id}:${field}`;
    clearTimeout(this.#pendingSecrets.get(key)?.timer);
    const send = () => {
      this.#pendingSecrets.delete(key);
      return native
        .call('secret', { id, field, value })
        .then(() => undefined)
        .catch(() => toast(t('saveFailed')));
    };
    this.#pendingSecrets.set(key, { timer: setTimeout(send, SECRET_DELAY_MS), send });
  }

  async pickKey(): Promise<void> {
    const connection = this.current;
    if (!connection) return;
    await this.#flush();
    try {
      const name = await native.callHidden<string | null>('pickKey', { id: connection.id });
      if (name === null) return;
      this.keyError = false;
      this.held[connection.id] = { ...this.heldOf(connection.id), key: true };
      Object.assign(connection, { keyName: name });
      this.saveNow();
    } catch {
      this.keyError = true;
    }
  }

  async connect(connection: Connection, openWhenOn = false): Promise<void> {
    if (openWhenOn) this.#openWhenOn.add(connection.id);
    await this.#flush();
    this.statuses[connection.id] = { state: 'busy' };
    native.call('connect', { id: connection.id }).catch(() => (this.statuses[connection.id] = OFF));
  }

  disconnect(connection: Connection): void {
    native.call('disconnect', { id: connection.id }).catch(() => toast(t('saveFailed')));
  }

  toggle(connection: Connection): void {
    if (this.statusOf(connection.id).state === 'on') this.disconnect(connection);
    else void this.connect(connection);
  }

  open(connection: Connection): void {
    native.call('open', { id: connection.id }).catch(() => undefined);
  }

  trust(connection: Connection): void {
    this.statuses[connection.id] = { state: 'busy' };
    native.call('trust', { id: connection.id }).catch(() => (this.statuses[connection.id] = OFF));
  }

  refuse(connection: Connection): void {
    this.statuses[connection.id] = OFF;
  }

  #changed(id: string, status: Status): void {
    const before = this.statusOf(id).state;
    this.statuses[id] = status;
    const connection = this.connections.find((other) => other.id === id);
    if (!connection) return;
    if (status.state === 'on' && before !== 'on') {
      const place = placeOf(connection);
      toast(place ? t('connected', { name: connection.name, place }) : t('connectedFolder', { name: connection.name }));
      if (this.#openWhenOn.delete(id)) this.open(connection);
    } else if (status.state === 'off' && before === 'busy') {
      toast(t('disconnected', { name: connection.name }));
    } else if (status.state !== 'busy') {
      this.#openWhenOn.delete(id);
    }
  }

  #saveSoon(): void {
    if (this.#pendingSave !== null) clearTimeout(this.#pendingSave);
    this.#pendingSave = setTimeout(() => this.saveNow(), SAVE_DELAY_MS);
  }

  /** What is still waiting reaches the program before it connects or opens a window. */
  async #flush(): Promise<void> {
    if (this.#pendingSave !== null) await this.saveNow();
    const secrets = [...this.#pendingSecrets.values()];
    for (const { timer } of secrets) clearTimeout(timer);
    await Promise.all(secrets.map(({ send }) => send()));
  }

  saveNow(): Promise<void> {
    if (this.#pendingSave !== null) clearTimeout(this.#pendingSave);
    this.#pendingSave = null;
    if (!this.#loaded) return Promise.resolve();
    return native
      .call('save', { connections: $state.snapshot(this.connections) })
      .then(() => undefined)
      .catch(() => toast(t('saveFailed')));
  }
}

export const remote = new Remote();
