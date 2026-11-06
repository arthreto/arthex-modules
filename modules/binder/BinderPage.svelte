<script lang="ts">
  import {
    Button,
    formatWhen,
    Icon,
    IconButton,
    Menu,
    MenuItem,
    Modal,
    Segmented,
    TextField,
    ToggleChip,
  } from '@arthex/sdk';
  import { onDestroy, onMount } from 'svelte';
  import BinderEditor from './BinderEditor.svelte';
  import BinderView from './BinderView.svelte';
  import { binder, type BinderDocument, type Category, type Mode } from './binder.svelte';
  import { ICONS } from './blocks';
  import { t } from './i18n';
  import { firstLine, taskCount } from './markdown';

  type NewKind = 'document' | 'category';
  type Dialog =
    | { kind: 'new' }
    | { kind: 'renameCategory'; category: Category }
    | { kind: 'renameDocument'; document: BinderDocument }
    | { kind: 'removeCategory'; category: Category; count: number }
    | { kind: 'removeDocument'; document: BinderDocument };
  type RightClickTarget =
    { kind: 'category'; category: Category; count: number } | { kind: 'document'; document: BinderDocument };
  type RightClick = RightClickTarget & { x: number; y: number };

  interface Group {
    id: string | null;
    name: string;
    folded: boolean;
    documents: BinderDocument[];
    category: Category | null;
  }

  onMount(() => {
    void binder.load();
  });
  onDestroy(() => void binder.flush());

  const current = $derived(binder.current);
  const modes = $derived<{ value: Mode; label: string }[]>([
    { value: 'view', label: t('view') },
    { value: 'edit', label: t('edit') },
  ]);

  let dragged = $state<string | null>(null);
  let dropOn = $state<string | null | undefined>(undefined);
  let dialog = $state<Dialog | null>(null);
  let dialogName = $state('');
  let newKind = $state<NewKind>('document');
  let newCategory = $state<string | null>(null);
  let rightClick = $state<RightClick | null>(null);

  const newKinds = $derived<{ value: NewKind; label: string }[]>([
    { value: 'document', label: t('create.document') },
    { value: 'category', label: t('create.category') },
  ]);
  const needsName = $derived(
    dialog?.kind === 'new' || dialog?.kind === 'renameCategory' || dialog?.kind === 'renameDocument',
  );

  const loose = $derived(binder.documents.filter((document) => document.category === null));
  const groups = $derived.by<Group[]>(() => {
    const list: Group[] = binder.categories.map((category) => ({
      id: category.id,
      name: category.name,
      folded: category.folded,
      category,
      documents: binder.documents.filter((document) => document.category === category.id),
    }));
    // Shown while dragging too, so that a document can leave its category.
    if (loose.length > 0 || dragged) {
      list.push({
        id: null,
        name: t('noCategory'),
        folded: binder.noCategoryFolded,
        category: null,
        documents: loose,
      });
    }
    return list;
  });

  function toggle(group: Group): void {
    if (group.id) void binder.fold(group.id, !group.folded);
    else binder.noCategoryFolded = !binder.noCategoryFolded;
  }

  function preview(document: BinderDocument): string {
    const tasks = taskCount(document.text);
    if (tasks.total > 0 && /^\s*[-*]\s+\[/.test(document.text)) {
      return t('tasksDone', { done: tasks.done, total: tasks.total });
    }
    return firstLine(document.text) || t('emptyPreview');
  }

  function drop(event: DragEvent, category: string | null): void {
    event.preventDefault();
    if (dragged) void binder.move(dragged, category);
    dragged = null;
    dropOn = undefined;
  }

  function ask(next: Dialog): void {
    rightClick = null;
    if (next.kind === 'renameCategory') dialogName = next.category.name;
    else if (next.kind === 'renameDocument') dialogName = next.document.title;
    else dialogName = '';
    dialog = next;
  }

  /** A new document goes where the user is, unless they say otherwise. */
  function askNew(category: string | null): void {
    newKind = 'document';
    newCategory = category;
    ask({ kind: 'new' });
  }

  function openMenu(event: MouseEvent, target: RightClickTarget): void {
    event.preventDefault();
    rightClick = { ...target, x: event.clientX, y: event.clientY };
  }

  const close = () => (dialog = null);

  function confirm(event: Event): void {
    event.preventDefault();
    const name = dialogName.trim();
    if (needsName && !name) return;
    if (dialog?.kind === 'new' && newKind === 'document') void binder.create(newCategory, name);
    else if (dialog?.kind === 'new') void binder.addCategory(name);
    else if (dialog?.kind === 'renameCategory') void binder.renameCategory(dialog.category.id, name);
    else if (dialog?.kind === 'renameDocument') void binder.renameDocument(dialog.document.id, name);
    else if (dialog?.kind === 'removeCategory') void binder.removeCategory(dialog.category.id);
    else if (dialog?.kind === 'removeDocument') void binder.remove(dialog.document);
    close();
  }

  const dialogTitle = $derived.by(() => {
    switch (dialog?.kind) {
      case 'new':
        return t('create.title');
      case 'renameCategory':
        return t('category.renameTitle');
      case 'renameDocument':
        return t('document.renameTitle');
      case 'removeCategory':
        return t('category.removeTitle', { name: dialog.category.name });
      case 'removeDocument':
        return t('document.deleteTitle', { title: dialog.document.title || t('untitled') });
      default:
        return '';
    }
  });
</script>

<div class="binder fade-in">
  <div class="list scroll">
    <div class="actions">
      <Button onclick={() => askNew(current?.category ?? null)}>
        <Icon name="plus" size={14} stroke={2.2} />{t('create.title')}
      </Button>
    </div>
    {#each groups as group (group.id)}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        class="group"
        class:drop={dropOn === group.id}
        oncontextmenu={(event) => {
          if (group.category) {
            openMenu(event, { kind: 'category', category: group.category, count: group.documents.length });
          }
        }}
        ondragover={(event) => {
          event.preventDefault();
          dropOn = group.id;
        }}
        ondragleave={() => (dropOn = undefined)}
        ondrop={(event) => drop(event, group.id)}
      >
        <button type="button" class="fold" aria-expanded={!group.folded} onclick={() => toggle(group)}>
          <span class="chevron" class:open={!group.folded}
            ><Icon path={ICONS.chevron} size={14} stroke={2.2} /></span
          >
          <span class="group-name">{group.name}</span>
          <span class="count">{group.documents.length}</span>
        </button>
      </div>
      {#if !group.folded}
        {#each group.documents as document (document.id)}
          <button
            type="button"
            class="entry"
            class:on={document.id === current?.id}
            draggable="true"
            aria-current={document.id === current?.id ? 'true' : undefined}
            ondragstart={(event) => {
              dragged = document.id;
              event.dataTransfer?.setData('text/plain', document.id);
            }}
            ondragend={() => {
              dragged = null;
              dropOn = undefined;
            }}
            oncontextmenu={(event) => openMenu(event, { kind: 'document', document })}
            onclick={() => binder.open(document.id)}
          >
            <span class="line">
              <span class="title">{document.title || t('untitled')}</span>
              <span class="when">{formatWhen(document.updated)}</span>
            </span>
            <span class="preview">{preview(document)}</span>
          </button>
        {/each}
      {/if}
    {/each}
  </div>
  {#if current}
    <section class="editor">
      <div class="head">
        <input
          class="name"
          aria-label={t('document.title')}
          placeholder={t('untitled')}
          value={current.title}
          oninput={(event) => binder.update({ title: event.currentTarget.value })}
        />
        <span class="status">
          {#if !binder.saving}<Icon name="check" size={12} stroke={2.4} />{/if}
          {t(binder.saving ? 'saving' : 'saved')}
        </span>
        <Segmented
          label={t('display')}
          options={modes}
          value={binder.mode}
          onchange={(mode) => (binder.mode = mode)}
        />
        <Button onclick={() => binder.exportPdf()}>{t('document.exportPdf')}</Button>
        <IconButton
          icon="trash"
          label={t('document.delete')}
          onclick={() => ask({ kind: 'removeDocument', document: current })}
        />
      </div>
      {#if binder.mode === 'view'}
        <BinderView text={current.text} />
      {:else}
        {#key current.id}<BinderEditor text={current.text} />{/key}
      {/if}
    </section>
  {/if}
</div>

{#if rightClick?.kind === 'category'}
  {@const { category, count } = rightClick}
  <Menu x={rightClick.x} y={rightClick.y} label={category.name} onclose={() => (rightClick = null)}>
    <MenuItem icon="plus" onclick={() => askNew(category.id)}>{t('menu.newDocument')}</MenuItem>
    <MenuItem icon="edit" onclick={() => ask({ kind: 'renameCategory', category })}
      >{t('menu.rename')}</MenuItem
    >
    <MenuItem icon="trash" danger onclick={() => ask({ kind: 'removeCategory', category, count })}>
      {t('menu.delete')}
    </MenuItem>
  </Menu>
{:else if rightClick?.kind === 'document'}
  {@const { document } = rightClick}
  <Menu
    x={rightClick.x}
    y={rightClick.y}
    label={document.title || t('untitled')}
    onclose={() => (rightClick = null)}
  >
    <MenuItem icon="edit" onclick={() => ask({ kind: 'renameDocument', document })}
      >{t('menu.rename')}</MenuItem
    >
    <MenuItem icon="trash" danger onclick={() => ask({ kind: 'removeDocument', document })}>
      {t('menu.delete')}
    </MenuItem>
  </Menu>
{/if}

{#if dialog}
  <Modal title={dialogTitle} onclose={close}>
    <form id="binder-dialog" onsubmit={confirm}>
      {#if dialog.kind === 'new'}
        <span class="kinds">
          <Segmented
            label={t('create.type')}
            options={newKinds}
            value={newKind}
            onchange={(kind) => (newKind = kind)}
          />
        </span>
      {/if}
      {#if needsName}
        <TextField
          label={t('category.name')}
          bind:value={dialogName}
          maxlength={dialog.kind === 'renameDocument' || newKind === 'document' ? 200 : 40}
          autofocus
        />
      {:else if dialog.kind === 'removeCategory'}
        <p>{dialog.count === 0 ? t('category.empty') : t('category.moves', { count: dialog.count })}</p>
      {:else}
        <p>{t('document.deleteText')}</p>
      {/if}
      {#if dialog.kind === 'new' && newKind === 'document'}
        <div class="choice">
          <span>{t('create.category')}</span>
          <div class="chips" role="group" aria-label={t('create.category')}>
            {#each binder.categories as category (category.id)}
              <ToggleChip
                label={category.name}
                pressed={newCategory === category.id}
                onclick={() => (newCategory = category.id)}
              />
            {/each}
            <ToggleChip
              label={t('noCategory')}
              pressed={newCategory === null}
              onclick={() => (newCategory = null)}
            />
          </div>
        </div>
      {/if}
    </form>
    {#snippet actions()}
      <Button onclick={close}>{t('cancel')}</Button>
      {#if dialog?.kind === 'removeCategory' || dialog?.kind === 'removeDocument'}
        <Button variant="quiet-danger" type="submit" form="binder-dialog">{t('document.deleteAction')}</Button
        >
      {:else}
        <Button variant="primary" type="submit" form="binder-dialog" disabled={!dialogName.trim()}>
          {dialog?.kind === 'new' ? t('category.create') : t('category.renameAction')}
        </Button>
      {/if}
    {/snippet}
  </Modal>
{/if}

<style>
  .binder {
    position: absolute;
    inset: 0;
    display: flex;
    gap: 20px;
  }

  .list {
    width: 290px;
    flex: none;
    padding-right: 6px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .actions {
    flex: none;
    margin-bottom: 8px;
    display: flex;
    gap: 6px;
  }

  .actions :global(.button) {
    flex: 1;
    min-width: 0;
  }

  .group {
    flex: none;
    height: 32px;
    margin-top: 4px;
    padding: 0 4px 0 2px;
    border-radius: 8px;
    display: flex;
    align-items: center;
    gap: 2px;
  }

  .group.drop {
    background: var(--surface-selected);
  }

  .fold {
    flex: 1;
    min-width: 0;
    height: 100%;
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12.5px;
    font-weight: 600;
    color: var(--text-muted);
    text-align: left;
  }

  .chevron {
    flex: none;
    display: flex;
    transition: transform var(--motion-fast);
  }

  .chevron.open {
    transform: rotate(90deg);
  }

  .group-name {
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .count {
    flex: none;
    font-weight: 400;
    color: var(--text-subtle);
  }

  .entry {
    flex: none;
    width: 100%;
    padding: 11px 12px;
    border-radius: 10px;
    display: flex;
    flex-direction: column;
    gap: 3px;
    text-align: left;
    transition: background-color var(--motion-fast);
  }

  .entry:hover {
    background: var(--surface-hover);
  }

  .entry.on {
    background: var(--surface-selected);
  }

  .line {
    width: 100%;
    display: flex;
    justify-content: space-between;
    gap: 8px;
  }

  .title,
  .preview {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .title {
    font-size: 14px;
    font-weight: 500;
  }

  .when {
    flex: none;
    font-size: 11.5px;
    color: var(--text-subtle);
  }

  .preview {
    font-size: 12.5px;
    color: var(--text-subtle);
  }

  .editor {
    flex: 1;
    min-width: 0;
    padding: 16px 22px 18px;
    border-radius: 16px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    background: var(--surface);
    border: 1px solid var(--border);
  }

  .head {
    flex: none;
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .name {
    flex: 1;
    min-width: 0;
    padding: 4px 0;
    font: inherit;
    font-size: 22px;
    font-weight: 600;
    color: var(--text);
    background: transparent;
    border: 0;
    outline: none;
  }

  .status {
    flex: none;
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 12px;
    color: var(--text-subtle);
  }

  form {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  form p {
    font-size: 14px;
  }

  .kinds {
    align-self: flex-start;
  }

  .choice {
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 13px;
    color: var(--text-muted);
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
</style>
