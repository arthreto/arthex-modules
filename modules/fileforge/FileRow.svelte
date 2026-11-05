<script lang="ts">
  import { Button, Icon, IconButton, formatBytes } from '@arthex/sdk';
  import { extensionOf, formatLabel, type Item, type Kind } from './formats';
  import { forge } from './forge.svelte';
  import { t } from './i18n';

  const CHEVRON = 'm7 10 5 5 5-5';
  const KINDS: Record<Kind, { icon: string; tint: string }> = {
    image: { icon: 'M4 5h16v14H4zM4 16l5-5 4 4 2-2 5 5M15 9h.01', tint: 'green' },
    video: { icon: 'M4 6h16v12H4zM10 9l5 3-5 3z', tint: 'sky' },
    audio: {
      icon: 'M9 18V6l10-2v12M9 18a3 3 0 1 1-6 0a3 3 0 0 1 6 0M19 16a3 3 0 1 1-6 0a3 3 0 0 1 6 0',
      tint: 'pink',
    },
    document: { icon: 'M6 3h9l4 4v14H6zM15 3v4h4M9 12h7M9 16h7', tint: 'orange' },
  };

  let { item, onformat }: { item: Item; onformat: (event: MouseEvent) => void } = $props();

  const kind = $derived(KINDS[item.kind]);
  const finished = $derived(item.status === 'done');
  const showBar = $derived(item.status !== 'ready');

  const stateText = $derived.by(() => {
    if (item.status === 'failed') return item.error ? t(`errors.${item.error}`) : t('errors.failed');
    if (item.status === 'done') return item.noGain ? t('noGain') : t('done');
    if (item.status === 'working') return `${item.progress} %`;
    if (item.status === 'waiting') return t('waiting');
    return '';
  });

  const thumbnail = $derived(forge.thumbnails[item.id]);

  const conversion = $derived.by(() => {
    if (item.status === 'ready') return null;
    if (item.mode === 'convert') return `${extensionOf(item.name)} → ${formatLabel(item.target)}`;
    return item.output ? `${t('modes.compress')} · ${extensionOf(item.output)}` : t('modes.compress');
  });

  const sizes = $derived(
    item.outputSize === null
      ? formatBytes(item.size)
      : `${formatBytes(item.size)} → ${formatBytes(item.outputSize)}`,
  );

  const detail = $derived(conversion ? `${conversion} · ${sizes}` : sizes);
</script>

<div class="row">
  {#if thumbnail}
    <img class="preview" src={thumbnail} alt="" />
  {:else}
    <span class="icon" style:color="var(--tint-{kind.tint})"><Icon path={kind.icon} size={20} /></span>
  {/if}
  <span class="text">
    <span class="line">
      <span class="name">{item.name}</span>
      <span class="state" class:good={finished} class:bad={item.status === 'failed'}>{stateText}</span>
    </span>
    <span class="detail">{detail}</span>
    {#if showBar}
      <span class="bar" aria-hidden="true">
        <span class:good={finished} style:width="{finished ? 100 : item.progress}%"></span>
      </span>
    {/if}
  </span>
  {#if item.status === 'ready' && forge.mode === 'convert'}
    <span class="from">{extensionOf(item.name)} →</span>
    <Button
      size="small"
      aria-haspopup="menu"
      aria-label={t('targetOf', { name: item.name, format: formatLabel(item.target) })}
      onclick={onformat}
    >
      {formatLabel(item.target)}
      <Icon path={CHEVRON} size={14} stroke={2} />
    </Button>
  {/if}
  {#if finished}
    <button
      class="folder"
      aria-label={t('reveal', { name: item.name })}
      title={t('reveal', { name: item.name })}
      onclick={() => forge.reveal(item.id)}
    >
      <Icon path="M3 7h6l2 2h10v10H3z" size={16} />
    </button>
  {:else if item.status !== 'working'}
    <IconButton icon="close" label={t('remove', { name: item.name })} onclick={() => forge.remove(item.id)} />
  {/if}
</div>

<style>
  .row {
    padding: 12px 0;
    display: flex;
    align-items: center;
    gap: 14px;
  }

  .row + :global(.row) {
    border-top: 1px solid var(--border);
  }

  .icon {
    width: 40px;
    height: 40px;
    flex: none;
    box-sizing: border-box;
    border-radius: 12px;
    background: var(--surface-raised);
    border: 1px solid var(--border-strong);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .preview {
    width: 40px;
    height: 40px;
    flex: none;
    border-radius: 12px;
    object-fit: cover;
  }

  .text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 5px;
  }

  .line {
    display: flex;
    align-items: baseline;
    gap: 10px;
  }

  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    font-size: 14px;
    font-weight: 600;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .state {
    font-size: 12.5px;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
    color: var(--text-subtle);
  }

  .state.good {
    color: var(--tint-green);
  }

  .state.bad {
    color: var(--tint-red);
  }

  .detail,
  .from {
    font-size: 12.5px;
    color: var(--text-subtle);
  }

  .from {
    flex: none;
  }

  .bar {
    display: block;
    height: 4px;
    overflow: hidden;
    border-radius: 2px;
    background: var(--surface-raised);
  }

  .bar > span {
    display: block;
    height: 100%;
    border-radius: 2px;
    background: var(--accent);
    transition: width 400ms linear;
  }

  .bar > .good {
    background: var(--tint-green);
  }

  .folder {
    width: 32px;
    height: 32px;
    flex: none;
    border-radius: 8px;
    color: var(--text-muted);
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background-color var(--motion-fast);
  }

  .folder:hover {
    background: var(--surface-hover);
    color: var(--text);
  }
</style>
