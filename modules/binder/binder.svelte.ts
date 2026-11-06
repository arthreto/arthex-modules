import { formatBytes, moduleNative, toast } from '@arthex/sdk';
import temml from 'temml';
import { t } from './i18n';
import { ID } from './identity';
import { documentHtml } from './paper';

const native = moduleNative(ID);

/** Typing is saved this long after the last key, and right away when leaving the document. */
const SAVE_DELAY_MS = 500;

export type Mode = 'view' | 'edit';
export type MediaKind = 'image' | 'video' | 'file';

export interface Category {
  id: string;
  name: string;
  folded: boolean;
}

export interface BinderDocument {
  id: string;
  title: string;
  category: string | null;
  updated: number;
  text: string;
}

export interface Imported {
  file: string;
  name: string;
  size: number;
}

interface State {
  categories: Category[];
  documents: BinderDocument[];
  sizes: Record<string, number>;
}

function failed(): void {
  toast(t('failed'));
}

class Binder {
  categories = $state<Category[]>([]);
  documents = $state<BinderDocument[]>([]);
  sizes = $state<Record<string, number>>({});
  currentId = $state<string | null>(null);
  mode = $state<Mode>('view');
  saving = $state(false);
  /** The group without a category folds for the moment only. */
  noCategoryFolded = $state(false);
  #loaded: Promise<void> | null = null;
  #timer: ReturnType<typeof setTimeout> | undefined;
  #pending: BinderDocument | null = null;

  get current(): BinderDocument | null {
    return this.documents.find((document) => document.id === this.currentId) ?? this.documents[0] ?? null;
  }

  load(): Promise<void> {
    this.#loaded ??= native
      .call<State>('state')
      .then((state) => {
        this.categories = state.categories;
        this.documents = state.documents;
        this.sizes = state.sizes;
        this.open(state.documents[0]?.id ?? null);
      })
      .catch(() => {
        this.#loaded = null;
        failed();
      });
    return this.#loaded;
  }

  open(id: string | null): void {
    void this.flush();
    this.currentId = id;
    this.mode = this.current?.text.trim() ? 'view' : 'edit';
  }

  async create(category: string | null, title: string): Promise<void> {
    await this.flush();
    const document: BinderDocument = { id: crypto.randomUUID(), title, category, updated: 0, text: '' };
    try {
      document.updated = await native.call<number>('saveDocument', document);
      this.documents = [document, ...this.documents];
      this.currentId = document.id;
      this.mode = 'edit';
      if (category) void this.fold(category, false);
      else this.noCategoryFolded = false;
    } catch {
      failed();
    }
  }

  /** Changes the open document at once on screen, and saves it shortly after. */
  update(patch: Partial<Pick<BinderDocument, 'title' | 'text'>>): void {
    const current = this.current;
    if (!current) return;
    const next = { ...current, ...patch };
    this.documents = this.documents.map((document) => (document.id === next.id ? next : document));
    this.#pending = next;
    this.saving = true;
    clearTimeout(this.#timer);
    this.#timer = setTimeout(() => void this.flush(), SAVE_DELAY_MS);
  }

  async flush(): Promise<void> {
    clearTimeout(this.#timer);
    const pending = this.#pending;
    this.#pending = null;
    if (!pending) return;
    try {
      const updated = await native.call<number>('saveDocument', pending);
      this.documents = this.documents.map((document) =>
        document.id === pending.id ? { ...document, updated } : document,
      );
    } catch {
      failed();
    } finally {
      this.saving = this.#pending !== null;
    }
  }

  async renameDocument(id: string, title: string): Promise<void> {
    await this.flush();
    const document = this.documents.find((other) => other.id === id);
    if (!document) return;
    try {
      const updated = await native.call<number>('saveDocument', { ...document, title });
      this.documents = this.documents.map((other) =>
        other.id === id ? { ...other, title, updated } : other,
      );
    } catch {
      failed();
    }
  }

  async remove(document: BinderDocument): Promise<void> {
    if (this.#pending?.id === document.id) {
      clearTimeout(this.#timer);
      this.#pending = null;
      this.saving = false;
    }
    try {
      await native.call('deleteDocument', { id: document.id });
      this.documents = this.documents.filter((other) => other.id !== document.id);
      this.open(this.documents[0]?.id ?? null);
      toast(t('document.deleted'));
    } catch {
      failed();
    }
  }

  async move(id: string, category: string | null): Promise<void> {
    await this.flush();
    try {
      await native.call('moveDocument', { id, category });
      this.documents = this.documents.map((document) =>
        document.id === id ? { ...document, category } : document,
      );
    } catch {
      failed();
    }
  }

  async addCategory(name: string): Promise<void> {
    const id = crypto.randomUUID();
    try {
      await native.call('saveCategory', { id, name });
      this.categories = [...this.categories, { id, name: name.trim(), folded: false }];
    } catch {
      failed();
    }
  }

  async renameCategory(id: string, name: string): Promise<void> {
    try {
      await native.call('saveCategory', { id, name });
      this.categories = this.categories.map((category) =>
        category.id === id ? { ...category, name: name.trim() } : category,
      );
    } catch {
      failed();
    }
  }

  /** Its documents stay, in the group without a category. */
  async removeCategory(id: string): Promise<void> {
    await this.flush();
    try {
      await native.call('deleteCategory', { id });
      this.categories = this.categories.filter((category) => category.id !== id);
      this.documents = this.documents.map((document) =>
        document.category === id ? { ...document, category: null } : document,
      );
    } catch {
      failed();
    }
  }

  async fold(id: string, folded: boolean): Promise<void> {
    this.categories = this.categories.map((category) =>
      category.id === id ? { ...category, folded } : category,
    );
    await native.call('foldCategory', { id, folded }).catch(failed);
  }

  /** The system's window to pick a file needs the overlay out of the way. */
  async pickMedia(kind: MediaKind): Promise<Imported | null> {
    try {
      return this.#imported(await native.callHidden<Imported | null>('pickMedia', { kind }));
    } catch {
      failed();
      return null;
    }
  }

  async pasteImage(data: string, extension: string): Promise<Imported | null> {
    try {
      return this.#imported(await native.call<Imported>('pasteImage', { data, extension }));
    } catch {
      failed();
      return null;
    }
  }

  #imported(imported: Imported | null): Imported | null {
    if (imported) this.sizes = { ...this.sizes, [imported.file]: imported.size };
    return imported;
  }

  openFile(file: string): void {
    native.call('openFile', { file }).catch(failed);
  }

  openLink(url: string): void {
    native.call('openLink', { url }).catch(failed);
  }

  /** What a file link shows and the paper says: its kind and its size. */
  fileDetails(file: string): string {
    const kind = file.split('.').at(-1)?.toUpperCase() ?? '';
    const size = this.sizes[file];
    return size === undefined ? kind : `${kind} · ${formatBytes(size)}`;
  }

  async exportPdf(): Promise<void> {
    const document = this.current;
    if (!document) return;
    await this.flush();
    const title = document.title || t('untitled');
    const html = documentHtml(title, document.text, {
      video: (name) => t('videoLine', { name }),
      fileSize: (file) => this.fileDetails(file),
      formula: (tex) => temml.renderToString(tex, { displayMode: true }),
    });
    try {
      const file = await native.callHidden<string | null>('exportPdf', { title, html });
      if (file) toast(t('document.exported', { file }));
    } catch {
      failed();
    }
  }
}

export const binder = new Binder();
