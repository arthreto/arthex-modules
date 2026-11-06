import { moduleNative, toast } from '@arthex/sdk';
import { dayOfDate, moved, zoomed, type Day, type Level } from './dates';
import { t } from './i18n';
import { ID } from './identity';
import { draftOf, itemOf, newDraft, type Draft, type Item } from './items';

const native = moduleNative(ID);

const momentOf = (date: Date) => `${dayOfDate(date)}T${date.toTimeString().slice(0, 5)}`;

/** What the page and the Home card share. It stays while the overlay closes, which rebuilds them. */
class Calendar {
  items = $state<Item[]>([]);
  level = $state<Level>('month');
  today = $state(dayOfDate(new Date()));
  now = $state(momentOf(new Date()));
  focus = $state(dayOfDate(new Date()));
  selected = $state(dayOfDate(new Date()));
  draft = $state<Draft | null>(null);

  async load(): Promise<void> {
    this.tick();
    try {
      this.items = await native.call<Item[]>('items');
    } catch {
      // The program is not there yet: a card or page stays empty until the next opening.
    }
  }

  tick(): void {
    const date = new Date();
    const today = dayOfDate(date);
    // Whoever looks at today stays on today when midnight passes.
    if (this.selected === this.today) this.selected = today;
    if (this.focus === this.today) this.focus = today;
    this.today = today;
    this.now = momentOf(date);
  }

  zoom(direction: 1 | -1): void {
    this.level = zoomed(this.level, direction);
  }

  move(direction: 1 | -1): void {
    this.focus = moved(this.level, this.focus, direction);
  }

  goToday(): void {
    this.focus = this.today;
    this.selected = this.today;
  }

  select(day: Day): void {
    this.selected = day;
  }

  /** From a small month: the month itself, on that day. */
  openMonth(day: Day): void {
    this.level = 'month';
    this.focus = day;
    this.selected = day;
  }

  add(day: Day = this.selected): void {
    this.draft = newDraft('event', day);
  }

  edit(item: Item): void {
    this.draft = draftOf(item);
  }

  close(): void {
    this.draft = null;
  }

  async save(draft: Draft): Promise<void> {
    const item = itemOf(draft, draft.id || crypto.randomUUID());
    if (!item) return;
    try {
      this.items = await native.call<Item[]>('save', item);
      this.draft = null;
    } catch {
      toast(t('editor.saveFailed'));
    }
  }

  async remove(id: string): Promise<void> {
    try {
      this.items = await native.call<Item[]>('remove', { id });
      this.draft = null;
    } catch {
      toast(t('editor.saveFailed'));
    }
  }
}

export const calendar = new Calendar();
