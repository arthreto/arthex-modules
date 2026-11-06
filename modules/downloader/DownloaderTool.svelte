<script lang="ts">
  import {
    Button,
    Icon,
    Segmented,
    Skeleton,
    TextField,
    ToggleChip,
    formatBytes,
    formatPercent,
    type ToolProps,
  } from '@arthex/sdk';
  import { onMount } from 'svelte';
  import { downloader } from './downloader.svelte';
  import { t } from './i18n';
  import { fileName, formatDuration } from './links';

  const MINUTE = 60;
  const CHECK = 'm5 12 5 5 9-10';

  let { input }: ToolProps = $props();

  onMount(() => {
    downloader.load();
    downloader.open(input);
  });

  const info = $derived(downloader.info);
  const status = $derived(downloader.status);
  const qualities = $derived(
    downloader.audio
      ? (info?.audios ?? []).map((audio) => ({
          value: audio.bitrate,
          label: t('bitrate', { value: audio.bitrate }),
          size: audio.size,
        }))
      : (info?.videos ?? []).map((video) => ({
          value: video.height,
          label: `${video.height}p`,
          size: video.size,
        })),
  );
  const byline = $derived([info?.author, info?.site].filter(Boolean).join(' · '));

  function timeLeft(seconds: number): string {
    return seconds >= 2 * MINUTE
      ? t('minutesLeft', { count: Math.round(seconds / MINUTE) })
      : t('secondsLeft', { count: Math.ceil(seconds) });
  }

  const progress = $derived.by(() => {
    const parts = [formatPercent(status.percent / 100, 0)];
    if (status.processing) parts.push(t('finishing'));
    if (status.speed !== null) parts.push(t('speed', { value: formatBytes(status.speed) }));
    if (status.eta !== null) parts.push(timeLeft(status.eta));
    return parts.join(' · ');
  });
</script>

<div class="tool">
  <form class="link" onsubmit={(event) => event.preventDefault()}>
    <TextField
      label={t('link')}
      hideLabel
      autofocus
      spellcheck={false}
      autocomplete="off"
      placeholder={t('linkHint')}
      disabled={downloader.busy}
      value={downloader.link}
      oninput={(event) => downloader.setLink(event.currentTarget.value)}
    />
  </form>

  {#if downloader.failure}
    <span class="failure" role="alert">{t(`failures.${downloader.failure}`)}</span>
  {/if}

  {#if downloader.phase === 'loading'}
    <div class="content" aria-busy="true">
      <div class="about">
        <Skeleton width="300px" height="169px" radius="12px" />
        <Skeleton width="80%" height="16px" />
        <Skeleton width="50%" height="12px" />
      </div>
      <div class="choices">
        <Skeleton width="170px" height="36px" radius="8px" />
        <Skeleton width="60%" height="30px" radius="8px" />
        <Skeleton width="100%" height="66px" radius="8px" />
      </div>
    </div>
  {:else if downloader.phase === 'ready' && info}
    <div class="content">
      <div class="about">
        <div class="thumbnail">
          {#if downloader.thumbnail}<img src={downloader.thumbnail} alt="" />{/if}
          {#if info.duration !== null}<span class="duration">{formatDuration(info.duration)}</span>{/if}
        </div>
        <span class="title">{info.title}</span>
        <span class="byline">{byline}</span>
      </div>

      <div class="choices">
        <Segmented
          label={t('type')}
          value={downloader.audio ? 'audio' : 'video'}
          options={[
            { value: 'video', label: t('video'), disabled: info.audioOnly || downloader.busy },
            { value: 'audio', label: t('audio'), disabled: downloader.busy },
          ]}
          onchange={(kind) => downloader.setAudio(kind === 'audio')}
        />

        <div class="group">
          <span class="label">{t('format')}</span>
          <div class="chips" role="group" aria-label={t('format')}>
            {#each downloader.formats as format (format)}
              <ToggleChip
                label={format.toUpperCase()}
                pressed={format === downloader.format}
                disabled={downloader.busy}
                onclick={() => downloader.setFormat(format)}
              />
            {/each}
          </div>
        </div>

        {#if downloader.hasQuality && qualities.length > 0}
          <div class="group">
            <span class="label">{t('quality')}</span>
            <div class="chips" role="group" aria-label={t('quality')}>
              {#each qualities as quality (quality.value)}
                <ToggleChip
                  label={quality.label}
                  detail={quality.size === null ? undefined : formatBytes(quality.size)}
                  pressed={quality.value === downloader.quality}
                  disabled={downloader.busy}
                  onclick={() => downloader.setQuality(quality.value)}
                />
              {/each}
            </div>
          </div>
        {/if}

        <div class="group">
          <span class="label">{t('folder')}</span>
          <div class="folder">
            <span class="path" title={downloader.folder}>{downloader.folder}</span>
            <Button disabled={downloader.busy} onclick={() => downloader.pickFolder()}>{t('change')}</Button>
          </div>
        </div>

        <div class="end">
          {#if downloader.busy}
            <div class="progress">
              <span
                class="bar"
                role="progressbar"
                aria-label={t('downloading')}
                aria-valuenow={Math.round(status.percent)}
                aria-valuemin="0"
                aria-valuemax="100"><span class="fill" style:width="{status.percent}%"></span></span
              >
              <span class="progress-text">{progress}</span>
            </div>
            <Button onclick={() => downloader.cancel()}>{t('cancel')}</Button>
          {:else if downloader.finished}
            <span class="file">
              <span class="check"><Icon path={CHECK} size={16} /></span>
              <span class="name">{fileName(downloader.finished)}</span>
            </span>
            <Button onclick={downloader.showFile}>{t('showInFolder')}</Button>
            <Button variant="primary" onclick={downloader.openFile}>{t('openFile')}</Button>
          {:else}
            <Button variant="primary" onclick={() => downloader.start()}>{t('download')}</Button>
          {/if}
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .tool {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 18px;
  }

  .link {
    flex: none;
  }

  .failure {
    font-size: 13.5px;
    color: var(--danger-text);
  }

  .content {
    flex: 1;
    min-height: 0;
    display: flex;
    gap: 28px;
  }

  .about {
    width: 300px;
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .thumbnail {
    position: relative;
    width: 300px;
    height: 169px;
    flex: none;
    overflow: hidden;
    border-radius: var(--radius-m);
    background: var(--surface-sunken);
    border: 1px solid var(--border);
  }

  .thumbnail img {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .duration {
    position: absolute;
    right: 8px;
    bottom: 8px;
    padding: 2px 6px;
    border-radius: 5px;
    background: var(--scrim);
    font-size: 12px;
    font-variant-numeric: tabular-nums;
  }

  .title {
    font-size: 15px;
    font-weight: 600;
    line-height: 1.35;
    overflow-wrap: anywhere;
  }

  .byline {
    font-size: 13px;
    color: var(--text-subtle);
  }

  .choices {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .group {
    display: flex;
    flex-direction: column;
    gap: 7px;
  }

  .label {
    font-size: 12.5px;
    color: var(--text-muted);
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .folder {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .path {
    flex: 1;
    min-width: 0;
    height: 36px;
    padding: 0 12px;
    line-height: 34px;
    border-radius: 10px;
    background: var(--surface-sunken);
    border: 1px solid var(--border-strong);
    font-size: 13px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .end {
    margin-top: auto;
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 12px;
    min-height: 40px;
  }

  .progress {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .bar {
    display: block;
    height: 7px;
    border-radius: 4px;
    background: var(--surface-hover);
    overflow: hidden;
  }

  .fill {
    display: block;
    height: 100%;
    border-radius: 4px;
    background: var(--accent);
    transition: width var(--motion-fast) linear;
  }

  .progress-text {
    font-size: 12.5px;
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
  }

  .file {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13.5px;
  }

  .check {
    display: flex;
    flex: none;
    color: var(--success);
  }

  .name {
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
