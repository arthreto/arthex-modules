<script lang="ts">
  import { Icon } from '@arthex/sdk';
  import { t } from './i18n';
  import { presence } from './presence.svelte';
  import { clock, progress, type Template } from './template';

  const IMAGE_ICON = 'M4 5h16v14H4zM4 16l5-5 4 4 2-2 5 5M15 9h.01';
  const IMAGE_DELAY_MS = 500;

  let { template }: { template: Template | null } = $props();

  const shownName = $derived(template?.title.trim() || t('appNameHint'));
  const large = $derived(template ? presence.imageOf(template.appId, template.large.trim()) : null);
  const small = $derived(template ? presence.imageOf(template.appId, template.small.trim()) : null);

  const timeText = $derived.by(() => {
    if (!template) return '';
    if (template.time === 'elapsed') return t('elapsedTime', { time: clock(0) });
    if (template.time === 'left' && template.minutes) {
      return t('leftTime', { time: clock(template.minutes * 60) });
    }
    if (template.time === 'dates' && template.from !== null && template.to !== null) {
      return `${clock((Date.now() - template.from) / 1000)} / ${clock((template.to - template.from) / 1000)}`;
    }
    return '';
  });

  const bar = $derived(template?.time === 'dates' ? progress(template.from, template.to, Date.now()) : null);

  const stateText = $derived.by(() => {
    if (!template) return '';
    const { state, size, max } = template;
    return size && max && size <= max ? t('partyCount', { state, size, max }) : state;
  });

  const statusText = $derived.by(() => {
    if (!template) return '';
    const shown = { name: shownName, details: template.details, state: template.state }[template.show];
    return t(`headings.${template.kind}`, { name: shown.trim() || shownName });
  });

  const link = $derived.by(() => {
    const current = presence.link;
    if (current.state !== 'refused') return t(`links.${current.state}`);
    if (template && presence.active === template.id && !template.appId) return t('links.noApp');
    return current.message ? t('links.refusedBecause', { message: current.message }) : t('links.refused');
  });

  // Typing a name or an address must not ask for an image at each key.
  $effect(() => {
    if (!template) return;
    const appId = template.appId;
    const values = [template.large.trim(), template.small.trim()];
    const timer = setTimeout(() => {
      for (const value of values) presence.requestImage(appId, value);
    }, IMAGE_DELAY_MS);
    return () => clearTimeout(timer);
  });
</script>

<aside class="side">
  <h4>{t('preview')}</h4>
  {#if template}
    <div class="card">
      <span class="heading">{t(`headings.${template.kind}`, { name: shownName })}</span>
      <div class="body">
        <span class="images">
          {#if large}
            <img class="large" src={large} alt="" />
          {:else}
            <span class="large empty"><Icon path={IMAGE_ICON} size={24} /></span>
          {/if}
          {#if small}<img class="small" src={small} alt="" />{/if}
        </span>
        <span class="lines">
          <span class="title">{shownName}</span>
          {#if template.details}<span>{template.details}</span>{/if}
          {#if stateText}<span>{stateText}</span>{/if}
          {#if timeText}<span class="time">{timeText}</span>{/if}
        </span>
      </div>
      {#if bar !== null}
        <span class="bar"><span style:width="{bar * 100}%"></span></span>
      {/if}
      {#each template.buttons.filter((button) => button.label.trim()) as button, index (index)}
        <span class="fake-button">{button.label}</span>
      {/each}
    </div>
    <span class="status">{statusText}</span>
  {:else}
    <div class="card" aria-hidden="true">
      <span class="ghost" style:width="55%" style:height="12px"></span>
      <div class="body">
        <span class="ghost large"></span>
        <span class="lines centred">
          <span class="ghost" style:width="80%" style:height="14px"></span>
          <span class="ghost" style:width="100%" style:height="11px"></span>
          <span class="ghost" style:width="60%" style:height="11px"></span>
        </span>
      </div>
    </div>
  {/if}
  <span class="link">
    <span class="dot {presence.link.state}" aria-hidden="true"></span>{link}
  </span>
</aside>

<style>
  .side {
    width: 280px;
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  h4 {
    margin: 0;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--text-subtle);
  }

  .card {
    padding: 14px;
    border-radius: 14px;
    background: var(--surface);
    border: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .heading {
    font-size: 12.5px;
    font-weight: 600;
    color: var(--text-muted);
    overflow-wrap: anywhere;
  }

  .body {
    min-width: 0;
    display: flex;
    gap: 12px;
  }

  .images {
    position: relative;
    width: 72px;
    height: 72px;
    flex: none;
  }

  .large {
    display: flex;
    width: 72px;
    height: 72px;
    flex: none;
    box-sizing: border-box;
    border-radius: 10px;
    object-fit: cover;
  }

  .empty {
    background: var(--surface-raised);
    border: 1px solid var(--border-strong);
    color: var(--text-subtle);
    align-items: center;
    justify-content: center;
  }

  .small {
    position: absolute;
    right: -6px;
    bottom: -6px;
    width: 24px;
    height: 24px;
    border-radius: 50%;
    object-fit: cover;
    border: 2px solid var(--surface-raised);
  }

  .lines {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
    font-size: 12.5px;
    color: var(--text-muted);
    overflow-wrap: anywhere;
  }

  .lines.centred {
    justify-content: center;
    gap: 8px;
  }

  .title {
    font-size: 14px;
    font-weight: 600;
    color: var(--text);
  }

  .time {
    color: var(--tint-green);
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
    background: var(--tint-green);
  }

  .fake-button {
    height: 32px;
    overflow: hidden;
    padding: 0 12px;
    border-radius: 10px;
    background: var(--surface-raised);
    border: 1px solid var(--border-strong);
    box-sizing: border-box;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 13px;
    font-weight: 500;
    white-space: nowrap;
  }

  .ghost {
    display: block;
    border-radius: 6px;
    background: var(--surface-raised);
  }

  .ghost.large {
    border-radius: 10px;
  }

  .status {
    font-size: 12.5px;
    color: var(--text-subtle);
    overflow-wrap: anywhere;
  }

  .link {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    color: var(--text-muted);
    overflow-wrap: anywhere;
  }

  .dot {
    width: 8px;
    height: 8px;
    flex: none;
    border-radius: 50%;
    background: var(--text-subtle);
  }

  .dot.connected {
    background: var(--tint-green);
  }

  .dot.offline {
    background: var(--tint-amber);
  }

  .dot.refused {
    background: var(--tint-red);
  }
</style>
