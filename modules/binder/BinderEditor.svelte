<script lang="ts">
  import { Button, Icon, IconButton } from '@arthex/sdk';
  import { tick } from 'svelte';
  import { binder, type Imported, type MediaKind } from './binder.svelte';
  import { ICONS, matchingBlocks, moved, type BlockId } from './blocks';
  import BlockMenu from './BlockMenu.svelte';
  import { caretPosition } from './caret';
  import { t } from './i18n';
  import { continueList, insertBlock, slashQuery, togglePrefix, wrap, type Edit } from './markdown';

  let { text }: { text: string } = $props();

  const MENU_HEIGHT = 340;
  const INSERT_MENU = { left: 12, top: 8 };
  /** Everything the clipboard can hold that a document can show. */
  const PASTED: Record<string, string> = {
    'image/png': 'png',
    'image/jpeg': 'jpg',
    'image/gif': 'gif',
    'image/webp': 'webp',
  };
  const PREFIXES: Partial<Record<BlockId, string>> = {
    h1: '# ',
    h2: '## ',
    h3: '### ',
    list: '- ',
    numbered: '1. ',
    task: '- [ ] ',
    quote: '> ',
  };
  const MEDIA: Partial<Record<BlockId, MediaKind>> = { image: 'image', video: 'video', file: 'file' };

  let area = $state<HTMLTextAreaElement>();
  let slash = $state<string | null>(null);
  let inserting = $state(false);
  let active = $state(0);
  let position = $state(INSERT_MENU);

  const menuOpen = $derived(inserting || slash !== null);
  const items = $derived(matchingBlocks(slash ?? '', (id) => t(`blocks.${id}`)));

  async function apply(edit: (text: string, start: number, end: number) => Edit) {
    const start = area?.selectionStart ?? text.length;
    const end = area?.selectionEnd ?? text.length;
    const result = edit(text, start, end);
    binder.update({ text: result.text });
    await tick();
    area?.focus();
    area?.setSelectionRange(result.start, result.end);
  }

  const mark = (open: string, sample: string, close = open) =>
    apply((text, start, end) => wrap(text, start, end, open, sample, close));
  const prefix = (value: string) => apply((text, start, end) => togglePrefix(text, start, end, value));

  const TOOLS = $derived([
    { icon: ICONS.bold, label: t('tools.bold'), run: () => mark('**', t('sample.bold')) },
    { icon: ICONS.italic, label: t('tools.italic'), run: () => mark('*', t('sample.italic')) },
    { icon: ICONS.strike, label: t('tools.strike'), run: () => mark('~~', t('sample.strike')) },
    { icon: ICONS.mark, label: t('tools.mark'), run: () => mark('==', t('sample.mark')) },
    { icon: ICONS.code, label: t('tools.code'), run: () => mark('`', t('sample.code')) },
    { icon: ICONS.link, label: t('tools.link'), run: () => mark('[', t('sample.link'), '](https://)') },
    null,
    { icon: ICONS.heading, label: t('tools.heading'), run: () => prefix('## ') },
    { icon: ICONS.list, label: t('tools.list'), run: () => prefix('- ') },
    { icon: ICONS.task, label: t('tools.task'), run: () => prefix('- [ ] ') },
    { icon: ICONS.quote, label: t('tools.quote'), run: () => prefix('> ') },
  ]);

  function closeMenu(): void {
    slash = null;
    inserting = false;
    active = 0;
  }

  function tableBlock(): string {
    const column = t('sample.column');
    return `| ${column} | ${column} | ${column} |\n| --- | --- | --- |\n|  |  |  |\n|  |  |  |`;
  }

  /** The text of a block, and where the caret goes in it. */
  function blockText(id: BlockId): { block: string; caretAt?: number } {
    switch (id) {
      case 'callout':
        return { block: '> [!NOTE] ' };
      case 'details':
        return {
          block: `<details><summary>${t('sample.question')}</summary>\n${t('sample.answer')}\n</details>`,
        };
      case 'code':
        return { block: '```\n\n```', caretAt: 4 };
      case 'math':
        return { block: '$$\n\n$$', caretAt: 3 };
      case 'table':
        return { block: tableBlock() };
      default:
        return { block: '---\n' };
    }
  }

  const mediaBlock = (kind: MediaKind, imported: Imported): string =>
    kind === 'file'
      ? `[${imported.name}](media/${imported.file})`
      : `![${imported.name}](media/${imported.file})`;

  async function insertMedia(kind: MediaKind, fromSlash: boolean) {
    const start = area?.selectionStart ?? text.length;
    const end = area?.selectionEnd ?? text.length;
    const imported = await binder.pickMedia(kind);
    if (imported) {
      await apply((text) =>
        insertBlock(text, start, end, `${mediaBlock(kind, imported)}\n`, { slash: fromSlash }),
      );
    }
  }

  function pick(id: BlockId): void {
    const fromSlash = slash !== null;
    closeMenu();
    const kind = MEDIA[id];
    const linePrefix = PREFIXES[id];
    if (kind) void insertMedia(kind, fromSlash);
    else if (linePrefix && fromSlash)
      void apply((text, start, end) => insertBlock(text, start, end, linePrefix, { slash: true }));
    else if (linePrefix) void prefix(linePrefix);
    else {
      const { block, caretAt } = blockText(id);
      void apply((text, start, end) => insertBlock(text, start, end, block, { slash: fromSlash, caretAt }));
    }
  }

  function placeMenu(): void {
    if (!area) return;
    const caret = caretPosition(area);
    const room = area.clientHeight - MENU_HEIGHT;
    position = {
      left: Math.min(caret.left, area.clientWidth - 240),
      top: Math.max(0, Math.min(caret.top, room)),
    };
  }

  function oninput(event: Event & { currentTarget: HTMLTextAreaElement }) {
    const { value, selectionStart } = event.currentTarget;
    binder.update({ text: value });
    const wasOpen = slash !== null;
    slash = slashQuery(value, selectionStart);
    inserting = false;
    active = 0;
    if (slash !== null && !wasOpen) placeMenu();
  }

  function onkeydown(event: KeyboardEvent & { currentTarget: HTMLTextAreaElement }) {
    if (menuOpen) {
      if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
        event.preventDefault();
        active = moved(active, event.key === 'ArrowDown' ? 1 : -1, items.length);
        return;
      }
      if (event.key === 'Enter') {
        const item = items[active];
        if (item) {
          event.preventDefault();
          pick(item.id);
          return;
        }
      }
      if (event.key === 'Escape') {
        // Otherwise it would close the overlay too.
        event.preventDefault();
        event.stopPropagation();
        closeMenu();
        return;
      }
    }
    const key = event.key.toLowerCase();
    if ((event.ctrlKey || event.metaKey) && (key === 'b' || key === 'i')) {
      event.preventDefault();
      void (key === 'b' ? mark('**', t('sample.bold')) : mark('*', t('sample.italic')));
    } else if (event.key === 'Enter' && !event.shiftKey) {
      const result = continueList(text, event.currentTarget.selectionStart);
      if (!result) return;
      event.preventDefault();
      void apply(() => result);
    }
  }

  function toggleInsert(): void {
    slash = null;
    active = 0;
    position = INSERT_MENU;
    inserting = !inserting;
  }

  function base64Of(file: File): Promise<string> {
    return new Promise((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = () => resolve(String(reader.result).split(',')[1] ?? '');
      reader.onerror = () => reject(new Error('unreadable'));
      reader.readAsDataURL(file);
    });
  }

  async function onpaste(event: ClipboardEvent) {
    const image = [...(event.clipboardData?.files ?? [])].find((file) => PASTED[file.type]);
    const extension = image && PASTED[image.type];
    if (!image || !extension) return;
    event.preventDefault();
    const imported = await binder.pasteImage(await base64Of(image), extension);
    if (imported) {
      await apply((text, start, end) => insertBlock(text, start, end, `${mediaBlock('image', imported)}\n`));
    }
  }

  /** Keeps the caret in the text while a toolbar button is pressed. */
  const keepFocus = (event: MouseEvent) => event.preventDefault();
</script>

<div class="toolbar" role="toolbar" aria-label={t('toolbar')}>
  {#each TOOLS as tool, index (index)}
    {#if tool}
      <IconButton path={tool.icon} label={tool.label} onmousedown={keepFocus} onclick={tool.run} />
    {:else}
      <span class="separator" aria-hidden="true"></span>
    {/if}
  {/each}
  <span class="separator" aria-hidden="true"></span>
  <span class="insert">
    <Button size="small" aria-expanded={inserting} onmousedown={keepFocus} onclick={toggleInsert}>
      <Icon path={ICONS.plus} size={14} stroke={2.2} />{t('insert')}
    </Button>
  </span>
</div>
<div class="writing">
  <!-- svelte-ignore a11y_autofocus -->
  <textarea
    bind:this={area}
    aria-label={t('content')}
    placeholder={t('placeholder')}
    spellcheck="false"
    value={text}
    autofocus
    {oninput}
    {onkeydown}
    {onpaste}
    onblur={closeMenu}></textarea>
  {#if menuOpen}
    <BlockMenu {items} {active} left={position.left} top={position.top} onpick={pick} />
  {/if}
</div>

<style>
  .toolbar {
    flex: none;
    padding: 4px;
    border-radius: 10px;
    align-self: flex-start;
    display: flex;
    align-items: center;
    gap: 2px;
    background: var(--surface-sunken);
  }

  .separator {
    width: 1px;
    height: 18px;
    margin: 0 4px;
    background: var(--border-strong);
  }

  .writing {
    position: relative;
    flex: 1;
    min-height: 0;
    display: flex;
  }

  textarea {
    flex: 1;
    min-width: 0;
    padding: 14px 16px;
    border-radius: 12px;
    resize: none;
    font: inherit;
    font-family: var(--font-mono);
    font-size: 13.5px;
    line-height: 1.6;
    color: var(--text);
    background: var(--surface-sunken);
    border: 1px solid var(--border);
    outline: none;
  }

  textarea:focus-visible {
    border-color: var(--accent);
  }
</style>
