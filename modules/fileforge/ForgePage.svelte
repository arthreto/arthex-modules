<script lang="ts">
  import { onMount } from 'svelte';
  import { Button, Icon, Panel, Segmented, formatBytes } from '@arthex/sdk';
  import FileRow from './FileRow.svelte';
  import FormatMenu from './FormatMenu.svelte';
  import { minutesLeft, type Item, type Mode } from './formats';
  import { forge } from './forge.svelte';
  import { t } from './i18n';
  import { ICON } from './identity';
  import SettingsDialog from './SettingsDialog.svelte';

  const LEVELS = { light: 80, balanced: 60, strong: 35 } as const;
  type Level = keyof typeof LEVELS;

  let settingsOpen = $state(false);
  let menu = $state<{ id: number; x: number; y: number } | null>(null);

  const menuItem = $derived(menu && forge.items.find((item) => item.id === menu?.id));
  const totals = $derived(forge.totals);
  const showTotal = $derived(totals.count > 0);
  const finished = $derived(showTotal && !totals.running);
  const readyCount = $derived(forge.items.filter((item) => item.status === 'ready').length);

  const modes = $derived(
    (['convert', 'compress'] as const).map((value) => ({ value, label: t(`modes.${value}`) })),
  );
  const levels = $derived(
    (Object.keys(LEVELS) as Level[]).map((value) => ({ value, label: t(`levels.${value}`) })),
  );
  const level = $derived(
    (Object.keys(LEVELS) as Level[]).find((name) => LEVELS[name] === forge.settings.level) ?? 'balanced',
  );

  const progressText = $derived.by(() => {
    if (finished) return t('finished');
    const current = Math.min(totals.count, totals.done + 1);
    const left = forge.startedAt === null ? null : minutesLeft(Date.now() - forge.startedAt, totals.percent);
    return left === null
      ? t('progressText', { done: current, count: totals.count })
      : t('progressTextLeft', { done: current, count: totals.count, minutes: left });
  });

  const summary = $derived(
    [
      t('qualityValue', { value: forge.settings.quality }),
      forge.settings.height === null ? t('original') : `${forge.settings.height}p`,
      forge.settings.folder ?? t('nextToOriginal'),
    ].join(' · '),
  );

  function openFormats(item: Item, event: MouseEvent) {
    const button = (event.currentTarget as HTMLElement).getBoundingClientRect();
    menu = { id: item.id, x: button.left, y: button.bottom + 6 };
  }

  function runMain() {
    if (totals.running) void forge.cancel();
    else if (readyCount > 0) void forge.start();
    else void forge.clear();
  }

  onMount(() => void forge.load());
</script>

<div class="page fade-in">
  <header>
    <h2>{forge.items.length === 0 ? t('name') : t('files', { count: forge.items.length })}</h2>
    <Segmented
      label={t('modeLabel')}
      options={modes}
      value={forge.mode}
      onchange={(mode: Mode) => forge.setMode(mode)}
    />
    <Button onclick={() => forge.addFiles()}
      ><Icon name="plus" size={14} stroke={2.2} />{t('addFiles')}</Button
    >
  </header>

  {#if showTotal}
    <Panel>
      <div class="total">
        <span class="figures">
          <span class="percent">{totals.percent} %</span>
          <span class="text">{progressText}</span>
          {#if finished && totals.saved > 0}
            <span class="saved">{t('saved', { size: formatBytes(totals.saved) })}</span>
          {/if}
        </span>
        <span
          class="bar"
          role="progressbar"
          aria-label={t('totalProgress')}
          aria-valuemin={0}
          aria-valuemax={100}
          aria-valuenow={totals.percent}
        >
          <span class:good={finished} style:width="{totals.percent}%"></span>
        </span>
      </div>
    </Panel>
  {/if}

  {#if forge.items.length === 0}
    <div class="empty">
      <span class="tile"><Icon path={ICON} size={32} stroke={1.6} /></span>
      <span class="empty-title">{t('noFiles')}</span>
      <Button variant="primary" onclick={() => forge.addFiles()}>{t('addFiles')}</Button>
    </div>
  {:else}
    <div class="list scroll">
      <Panel list>
        {#each forge.items as item (item.id)}
          <FileRow {item} onformat={(event) => openFormats(item, event)} />
        {/each}
      </Panel>
    </div>
  {/if}

  <div class="actions">
    <span class="summary">{summary}</span>
    {#if forge.mode === 'compress'}
      <Segmented
        label={t('levelLabel')}
        options={levels}
        value={level}
        onchange={(value: Level) => forge.setSettings({ level: LEVELS[value] })}
      />
    {/if}
    <Button onclick={() => (settingsOpen = true)}>
      <Icon path="M4 7h10M18 7h2M4 17h2M10 17h10M14 5v4M6 15v4" size={14} />{t('settings')}
    </Button>
    <Button
      variant={totals.running ? 'default' : 'primary'}
      disabled={forge.items.length === 0}
      onclick={runMain}
    >
      {#if totals.running}
        {t('cancel')}
      {:else if readyCount > 0}
        {forge.mode === 'convert'
          ? t('convertCount', { count: readyCount })
          : t('compressCount', { count: readyCount })}
      {:else}
        {t('newRun')}
      {/if}
    </Button>
  </div>
</div>

{#if menu && menuItem}
  <FormatMenu item={menuItem} x={menu.x} y={menu.y} onclose={() => (menu = null)} />
{/if}

{#if settingsOpen}
  <SettingsDialog onclose={() => (settingsOpen = false)} />
{/if}

<style>
  .page {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  header {
    min-height: 36px;
    display: flex;
    align-items: center;
    gap: 12px;
  }

  h2 {
    flex: 1;
    min-width: 0;
    margin: 0;
    font-size: 20px;
    font-weight: 600;
  }

  .total {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .figures {
    display: flex;
    align-items: baseline;
    gap: 10px;
  }

  .percent {
    font-size: 22px;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
  }

  .text {
    flex: 1;
    min-width: 0;
    font-size: 13px;
    color: var(--text-muted);
  }

  .saved {
    font-size: 13px;
    color: var(--tint-green);
  }

  .bar {
    display: block;
    height: 8px;
    overflow: hidden;
    border-radius: 4px;
    background: var(--surface-raised);
  }

  .bar > span {
    display: block;
    height: 100%;
    border-radius: 4px;
    background: var(--accent);
    transition: width 400ms linear;
  }

  .bar > .good {
    background: var(--tint-green);
  }

  .empty {
    flex: 1;
    min-height: 0;
    border-radius: 16px;
    border: 1.5px dashed var(--border-strong);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
  }

  .tile {
    width: 72px;
    height: 72px;
    box-sizing: border-box;
    border-radius: 20px;
    background: var(--surface-raised);
    border: 1px solid var(--border-strong);
    color: var(--tint-orange);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .empty-title {
    font-size: 18px;
    font-weight: 600;
  }

  .list {
    flex: 1;
    min-height: 0;
  }

  .actions {
    padding: 10px 12px 10px 18px;
    border-radius: 14px;
    background: var(--surface);
    border: 1px solid var(--border);
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .summary {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    font-size: 13px;
    white-space: nowrap;
    text-overflow: ellipsis;
    color: var(--text-muted);
  }
</style>
