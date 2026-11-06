<script lang="ts">
  import { Button, Check, Icon, IconButton, toast } from '@arthex/sdk';
  import { binder } from './binder.svelte';
  import { ICONS } from './blocks';
  import Formula from './Formula.svelte';
  import { t } from './i18n';
  import { parse, toggleTask, type Block, type Span, type Tone } from './markdown';
  import { mediaUrl } from './media';

  let { text }: { text: string } = $props();

  const blocks = $derived(parse(text));
  const TONE_ICONS: Record<Tone, string> = { note: ICONS.note, tip: ICONS.tip, warning: ICONS.warning };
  let broken = $state<string[]>([]);

  function follow(href: string): void {
    if (href.startsWith('media/')) binder.openFile(href.slice('media/'.length));
    else binder.openLink(href);
  }

  async function copy(code: string) {
    try {
      await navigator.clipboard.writeText(code);
      toast(t('copied'));
    } catch {
      toast(t('failed'));
    }
  }

  const isOpenable = (href: string): boolean => /^(https?:\/\/|media\/)/.test(href);
</script>

{#snippet spans(list: Span[])}
  {#each list as span, position (position)}
    {#if span.href && isOpenable(span.href)}
      {@const href = span.href}
      <button type="button" class="link" onclick={() => follow(href)}>{span.text}</button>
    {:else}
      <span
        class:bold={span.bold}
        class:italic={span.italic}
        class:strike={span.strike}
        class:code={span.code}
        class:mark={span.mark}>{span.text}</span
      >
    {/if}
  {/each}
{/snippet}

{#snippet list(items: Block[])}
  {#each items as block (block.line)}
    {#if block.kind === 'gap'}
      <div class="gap"></div>
    {:else if block.kind === 'hr'}
      <div class="rule" role="separator"></div>
    {:else if block.kind === 'task'}
      <button
        type="button"
        class="task"
        class:done={block.done}
        role="checkbox"
        aria-checked={block.done}
        onclick={() => binder.update({ text: toggleTask(text, block.line) })}
      >
        <Check checked={block.done} />
        <span>{@render spans(block.spans)}</span>
      </button>
    {:else if block.kind === 'li' || block.kind === 'ol'}
      <div class="item" style:padding-left="{4 + block.depth * 22}px">
        <span class="marker" aria-hidden="true">{block.kind === 'ol' ? `${block.number}.` : '•'}</span>
        <span>{@render spans(block.spans)}</span>
      </div>
    {:else if block.kind === 'quote'}
      <blockquote>{@render spans(block.spans)}</blockquote>
    {:else if block.kind === 'callout'}
      <div class="callout {block.tone}">
        <Icon path={TONE_ICONS[block.tone]} size={17} stroke={2} />
        <span>{@render spans(block.spans)}</span>
      </div>
    {:else if block.kind === 'math'}
      <Formula tex={block.tex} />
    {:else if block.kind === 'code'}
      <div class="code-block">
        <span class="copy"
          ><IconButton path={ICONS.copy} label={t('copy')} size={26} onclick={() => copy(block.code)} /></span
        >
        <pre>{block.code}</pre>
      </div>
    {:else if block.kind === 'table'}
      <div class="table scroll">
        <table>
          <thead>
            <tr>
              {#each block.head as cell, column (column)}
                <th>{@render spans(cell)}</th>
              {/each}
            </tr>
          </thead>
          <tbody>
            {#each block.rows as row, index (index)}
              <tr>
                {#each row as cell, column (column)}
                  <td>{@render spans(cell)}</td>
                {/each}
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {:else if block.kind === 'image'}
      {#if broken.includes(block.file)}
        <div class="missing">{t('missing', { name: block.caption || block.file })}</div>
      {:else}
        <figure>
          <img
            src={mediaUrl(block.file)}
            alt={block.caption}
            onerror={() => (broken = [...broken, block.file])}
          />
          {#if block.caption}
            <figcaption>{block.caption}</figcaption>
          {/if}
        </figure>
      {/if}
    {:else if block.kind === 'video'}
      {#if broken.includes(block.file)}
        <div class="missing">{t('missing', { name: block.caption || block.file })}</div>
      {:else}
        <!-- svelte-ignore a11y_media_has_caption -->
        <video
          controls
          preload="metadata"
          aria-label={block.caption}
          src={mediaUrl(block.file)}
          onerror={() => (broken = [...broken, block.file])}
        ></video>
      {/if}
    {:else if block.kind === 'file'}
      <div class="file">
        <span class="tile"><Icon path={ICONS.file} size={18} /></span>
        <span class="file-text">
          <span class="file-name">{block.name}</span>
          <span class="file-details">{binder.fileDetails(block.file)}</span>
        </span>
        <Button size="small" onclick={() => binder.openFile(block.file)}>{t('open')}</Button>
      </div>
    {:else if block.kind === 'details'}
      <details>
        <summary
          ><Icon path={ICONS.chevron} size={14} stroke={2.2} /><span>{@render spans(block.summary)}</span
          ></summary
        >
        <div class="folded">{@render list(block.body)}</div>
      </details>
    {:else}
      <div class={block.kind}>{@render spans(block.spans)}</div>
    {/if}
  {/each}
{/snippet}

<div class="view scroll">
  {#if blocks.length > 0}
    {@render list(blocks)}
  {:else}
    <div class="empty">
      {t('empty')}
      <Button onclick={() => (binder.mode = 'edit')}>{t('start')}</Button>
    </div>
  {/if}
</div>

<style>
  .view {
    flex: 1;
    min-height: 0;
    padding: 2px 10px 6px 2px;
    overflow-wrap: anywhere;
  }

  .h1 {
    margin: 6px 0 8px;
    font-size: 22px;
    font-weight: 700;
    line-height: 1.3;
    letter-spacing: -0.01em;
  }

  .h2 {
    margin: 12px 0 6px;
    font-size: 17.5px;
    font-weight: 650;
    line-height: 1.35;
  }

  .h3 {
    margin: 12px 0 4px;
    font-size: 12.5px;
    font-weight: 700;
    line-height: 1.4;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-subtle);
  }

  .p,
  .item {
    font-size: 14.5px;
    line-height: 1.6;
  }

  .p {
    margin: 0 0 6px;
  }

  .item {
    margin: 0 0 3px;
    display: flex;
    align-items: baseline;
    gap: 10px;
  }

  .marker {
    min-width: 14px;
    flex: none;
    font-weight: 600;
    color: var(--accent);
  }

  .task {
    width: calc(100% + 8px);
    margin-left: -8px;
    padding: 6px 8px;
    border-radius: 8px;
    display: flex;
    align-items: center;
    gap: 12px;
    font-size: 14.5px;
    line-height: 1.5;
    text-align: left;
    transition: background-color var(--motion-fast);
  }

  .task:hover {
    background: var(--surface-hover);
  }

  .task.done > span:last-child {
    text-decoration: line-through;
    color: var(--text-subtle);
  }

  .gap {
    height: 10px;
  }

  .rule {
    height: 1px;
    margin: 12px 0;
    background: var(--border-strong);
  }

  blockquote {
    margin: 2px 0 8px;
    padding: 2px 0 2px 14px;
    border-left: 3px solid var(--border-strong);
    font-size: 14.5px;
    font-style: italic;
    line-height: 1.6;
    color: var(--text-subtle);
  }

  .callout {
    margin: 2px 0 8px;
    padding: 10px 14px;
    border-radius: 10px;
    display: flex;
    gap: 10px;
    font-size: 14px;
    line-height: 1.55;
    background: var(--surface);
    border: 1px solid var(--border-strong);
  }

  .callout :global(svg) {
    flex: none;
    margin-top: 2px;
  }

  .callout.note :global(svg) {
    color: var(--tint-blue);
  }

  .callout.tip :global(svg) {
    color: var(--tint-green);
  }

  .callout.warning :global(svg) {
    color: var(--tint-amber);
  }

  .bold {
    font-weight: 700;
  }

  .italic {
    font-style: italic;
  }

  .strike {
    text-decoration: line-through;
  }

  .code {
    padding: 1px 5px;
    border-radius: 4px;
    font-family: var(--font-mono);
    background: var(--surface-hover);
  }

  .mark {
    padding: 1px 3px;
    border-radius: 4px;
    background: color-mix(in srgb, var(--tint-amber) 28%, transparent);
  }

  .link {
    color: var(--accent);
    text-decoration: underline;
    overflow-wrap: anywhere;
    text-align: left;
  }

  .code-block {
    position: relative;
    margin: 4px 0 10px;
    padding: 12px 14px;
    border-radius: 10px;
    background: var(--surface-sunken);
    border: 1px solid var(--border);
  }

  .copy {
    position: absolute;
    right: 6px;
    top: 6px;
  }

  pre {
    margin: 0;
    font-family: var(--font-mono);
    font-size: 13px;
    line-height: 1.55;
    white-space: pre-wrap;
  }

  .table {
    margin: 6px 0 12px;
  }

  table {
    width: 100%;
    border-collapse: collapse;
  }

  th,
  td {
    padding: 7px 12px;
    border: 1px solid var(--border-strong);
    font-size: 13.5px;
    line-height: 1.45;
    text-align: left;
    vertical-align: top;
  }

  th {
    font-weight: 600;
    background: var(--surface);
  }

  figure {
    margin: 6px 0 12px;
    text-align: center;
  }

  img {
    max-width: 100%;
    max-height: 420px;
    border-radius: 10px;
  }

  figcaption {
    margin-top: 6px;
    font-size: 12.5px;
    color: var(--text-subtle);
  }

  video {
    display: block;
    width: 100%;
    max-height: 420px;
    margin: 6px 0 12px;
    border-radius: 10px;
    background: #000000;
  }

  .missing {
    margin: 2px 0 8px;
    font-size: 13px;
    color: var(--text-subtle);
  }

  .file {
    margin: 4px 0 10px;
    padding: 10px 12px;
    border-radius: 10px;
    display: flex;
    align-items: center;
    gap: 12px;
    background: var(--surface);
    border: 1px solid var(--border-strong);
  }

  .tile {
    width: 36px;
    height: 36px;
    flex: none;
    border-radius: 9px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--tint-red);
    background: var(--surface-raised);
    border: 1px solid var(--border-strong);
  }

  .file-text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .file-name {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    font-size: 14px;
    font-weight: 500;
  }

  .file-details {
    font-size: 12px;
    color: var(--text-subtle);
  }

  details {
    margin: 2px 0 8px;
  }

  summary {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 14.5px;
    font-weight: 500;
    line-height: 1.6;
    cursor: pointer;
    list-style: none;
  }

  summary :global(svg) {
    flex: none;
    transition: transform var(--motion-fast);
  }

  details[open] > summary :global(svg) {
    transform: rotate(90deg);
  }

  .folded {
    margin: 2px 0 0 20px;
  }

  .empty {
    padding: 48px 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 14px;
    font-size: 14px;
    color: var(--text-muted);
  }
</style>
