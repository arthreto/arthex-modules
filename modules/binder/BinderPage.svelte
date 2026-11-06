<script lang="ts">
  import { Button, formatWhen, Icon, IconButton, Modal, Segmented, TextField } from '@arthex/sdk';
  import { onDestroy, onMount } from 'svelte';
  import BinderEditor from './BinderEditor.svelte';
  import BinderView from './BinderView.svelte';
  import { binder, type BinderDocument, type Category, type Mode } from './binder.svelte';
  import { ICONS } from './blocks';
  import { t } from './i18n';
  import { firstLine, taskCount } from './markdown';

  type Dialog =
    | { kind: 'new' }
    | { kind: 'rename'; category: Category }
    | { kind: 'removeCategory'; category: Category; count: number }
    | { kind: 'removeDocument'; document: BinderDocument };

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
    dialogName = next.kind === 'rename' ? next.category.name : '';
    dialog = next;
  }

  const close = () => (dialog = null);

  function confirm(event: Event): void {
    event.preventDefault();
    const name = dialogName.trim();
    if ((dialog?.kind === 'new' || dialog?.kind === 'rename') && !name) return;
    if (dialog?.kind === 'new' && name) void binder.addCategory(name);
    else if (dialog?.kind === 'rename' && name) void binder.renameCategory(dialog.category.id, name);
    else if (dialog?.kind === 'removeCategory') void binder.removeCategory(dialog.category.id);
    else if (dialog?.kind === 'removeDocument') void binder.remove(dialog.document);
    close();
  }

  const dialogTitle = $derived.by(() => {
    switch (dialog?.kind) {
      case 'new':
        return t('newCategory');
      case 'rename':
        return t('category.renameTitle');
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
      <span class="new">
        <Button onclick={() => binder.create(current?.category ?? null)}>
          <Icon name="plus" size={14} stroke={2.2} />{t('new')}
        </Button>
      </span>
      <IconButton
        path={ICONS.folderPlus}
        label={t('newCategory')}
        size={36}
        onclick={() => ask({ kind: 'new' })}
      />
    </div>
    {#each groups as group (group.id)}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        class="group"
        class:drop={dropOn === group.id}
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
        {#if group.category}
          {@const category = group.category}
          <span class="tools">
            <IconButton
              path={ICONS.plus}
              label={t('category.add', { name: group.name })}
              size={24}
              onclick={() => binder.create(category.id)}
            />
            <IconButton
              path={ICONS.pencil}
              label={t('category.rename', { name: group.name })}
              size={24}
              onclick={() => ask({ kind: 'rename', category })}
            />
            <IconButton
              icon="trash"
              label={t('category.remove', { name: group.name })}
              size={24}
              onclick={() => ask({ kind: 'removeCategory', category, count: group.documents.length })}
            />
          </span>
        {/if}
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

{#if dialog}
  <Modal title={dialogTitle} onclose={close}>
    <form id="binder-dialog" onsubmit={confirm}>
      {#if dialog.kind === 'new' || dialog.kind === 'rename'}
        <TextField label={t('category.name')} bind:value={dialogName} maxlength={40} autofocus />
      {:else if dialog.kind === 'removeCategory'}
        <p>{dialog.count === 0 ? t('category.empty') : t('category.moves', { count: dialog.count })}</p>
      {:else}
        <p>{t('document.deleteText')}</p>
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

  .new {
    flex: 1;
    min-width: 0;
    display: flex;
  }

  .new :global(.button) {
    width: 100%;
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

  .tools {
    flex: none;
    display: flex;
    gap: 1px;
    opacity: 0;
  }

  .group:hover .tools,
  .group:focus-within .tools {
    opacity: 1;
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

  form p {
    font-size: 14px;
  }
</style>
