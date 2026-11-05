import { moduleNative, toast } from '@arthex/sdk';
import { t } from './i18n';
import { ID } from './identity';
import { newTemplate, type Kind, type Template } from './template';

export type Link = { state: 'idle' | 'connected' | 'offline' } | { state: 'refused'; message: string };

interface ProgramState {
  templates: Template[];
  active: string | null;
  link: Link;
}

interface ImageEvent {
  appId: string;
  value: string;
  url: string | null;
}

const SAVE_DELAY_MS = 300;

const native = moduleNative(ID);
const imageKey = (appId: string, value: string) => `${appId}|${value}`;
const newId = () => `${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`;

class Presence {
  templates = $state<Template[]>([]);
  active = $state<string | null>(null);
  selected = $state<string | null>(null);
  link = $state<Link>({ state: 'idle' });
  images = $state<Record<string, string | null>>({});
  current = $derived(this.templates.find((template) => template.id === this.selected) ?? null);

  /** Nothing is saved before the program has answered, or an empty list would replace its own. */
  #loaded = false;
  #pendingSave: ReturnType<typeof setTimeout> | null = null;

  /** Each opening of the page asks for the link again: its events wait while the page is suspended. */
  async load(): Promise<void> {
    try {
      const state = await native.call<ProgramState>('state');
      this.link = state.link;
      if (this.#loaded) return;
      this.templates = state.templates;
      this.active = state.active;
      this.selected = state.active ?? state.templates[0]?.id ?? null;
      this.#loaded = true;
      native.on('status', (payload) => (this.link = payload as Link));
      native.on('image', (payload) => {
        const { appId, value, url } = payload as ImageEvent;
        this.images[imageKey(appId, value)] = url;
      });
    } catch {
      toast(t('loadFailed'));
    }
  }

  select(id: string): void {
    this.selected = id;
  }

  create(kind: Kind): void {
    const template = newTemplate(newId(), t('newName'), kind);
    this.templates.push(template);
    this.selected = template.id;
    this.#saveSoon();
  }

  duplicate(template: Template): void {
    const copy = { ...$state.snapshot(template), id: newId(), name: t('copyName', { name: template.name }) };
    this.templates.push(copy);
    this.selected = copy.id;
    this.#saveSoon();
  }

  remove(template: Template): void {
    this.templates = this.templates.filter((other) => other.id !== template.id);
    if (this.selected === template.id) this.selected = null;
    if (this.active === template.id) this.active = null;
    this.saveNow();
    toast(t('removed', { name: template.name }));
  }

  /** Changes the selected template. The program updates Discord if it is the active one. */
  update(changes: Partial<Template>): void {
    if (!this.current) return;
    Object.assign(this.current, changes);
    this.#saveSoon();
  }

  async toggle(template: Template): Promise<void> {
    const turningOn = this.active !== template.id;
    this.active = turningOn ? template.id : null;
    this.saveNow();
    try {
      await native.call('activate', { id: this.active });
      toast(turningOn ? t('turnedOn', { name: template.name }) : t('turnedOff'));
    } catch {
      toast(t('saveFailed'));
    }
  }

  imageOf(appId: string, value: string): string | null {
    return this.images[imageKey(appId, value)] ?? null;
  }

  requestImage(appId: string, value: string): void {
    const key = imageKey(appId, value);
    if (!value || key in this.images) return;
    this.images[key] = null;
    native.call('preview', { appId, value }).catch(() => undefined);
  }

  #saveSoon(): void {
    if (this.#pendingSave !== null) clearTimeout(this.#pendingSave);
    this.#pendingSave = setTimeout(() => this.saveNow(), SAVE_DELAY_MS);
  }

  saveNow(): void {
    if (this.#pendingSave !== null) clearTimeout(this.#pendingSave);
    this.#pendingSave = null;
    if (!this.#loaded) return;
    native.call('save', { templates: $state.snapshot(this.templates) }).catch(() => toast(t('saveFailed')));
  }
}

export const presence = new Presence();
