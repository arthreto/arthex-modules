import { moduleNative, toast } from '@arthex/sdk';
import { totals, type Item, type Kind, type Mode, type QueueState, type Settings } from './formats';
import { t } from './i18n';
import { ID } from './identity';

const native = moduleNative(ID);

class Forge {
  items = $state<Item[]>([]);
  mode = $state<Mode>('convert');
  settings = $state<Settings>({ quality: 80, height: null, stripMetadata: false, level: 60, folder: null });
  totals = $derived(totals(this.items));
  /** When the current run began, for the time left. */
  startedAt = $state<number | null>(null);
  /** Small pictures of the images, by file. */
  thumbnails = $state<Record<number, string>>({});
  #listening = false;

  #apply = (state: QueueState) => {
    this.items = state.items;
    this.mode = state.mode;
    this.settings = state.settings;
    if (!this.totals.running) this.startedAt = null;
    else this.startedAt ??= Date.now();
  };

  #call(method: string, params?: unknown): Promise<void> {
    return native.call<QueueState>(method, params).then(this.#apply, () => toast(t('failedAction')));
  }

  load(): Promise<void> {
    if (!this.#listening) {
      this.#listening = true;
      native.on('state', (state) => this.#apply(state as QueueState));
      native.on('thumbnail', (payload) => {
        const { id, url } = payload as { id: number; url: string };
        this.thumbnails[id] = url;
      });
      void native
        .call<Record<number, string>>('thumbnails')
        .then((known) => (this.thumbnails = { ...known, ...this.thumbnails }))
        .catch(() => undefined);
    }
    return this.#call('state');
  }

  /** The file dialog would open behind the overlay. */
  addFiles(): Promise<void> {
    return native.callHidden<QueueState>('pickFiles').then(this.#apply, () => toast(t('failedAction')));
  }

  pickFolder(): Promise<void> {
    return native.callHidden<QueueState>('pickFolder').then(this.#apply, () => toast(t('failedAction')));
  }

  setMode = (mode: Mode) => this.#call('setMode', { mode });
  setTarget = (id: number, target: string) => this.#call('setTarget', { id, target });
  setKindTarget = (kind: Kind, target: string) => this.#call('setKindTarget', { kind, target });
  setSettings = (changes: Partial<Settings>) =>
    this.#call('setSettings', { ...$state.snapshot(this.settings), ...changes });
  start = () => this.#call('start');
  cancel = () => this.#call('cancel');
  remove = (id: number) => this.#call('remove', { id });
  clear = () => this.#call('clear');

  reveal(id: number): void {
    native.call('reveal', { id }).catch(() => toast(t('failedAction')));
  }
}

export const forge = new Forge();
