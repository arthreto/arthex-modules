<script lang="ts">
  import {
    Button,
    IconButton,
    Segmented,
    ToggleChip,
    formatBytes,
    getLocale,
    type ToolProps,
  } from '@arthex/sdk';
  import { onDestroy, onMount } from 'svelte';
  import { FORMATS, LANGUAGES, SPEEDS, VOICES, clock, speedLabel, type Voice } from './choices';
  import { t } from './i18n';
  import { tts } from './tts.svelte';

  const PLAY = 'M7 4v16l13-8z';
  const PAUSE = 'M8 5v14M16 5v14';

  let { input }: ToolProps = $props();

  onMount(() => tts.load(input));
  onDestroy(() => tts.stop());

  const locale = getLocale();
  const model = $derived(tts.status?.model);
  const percent = $derived(Math.round((tts.speaking ? tts.status?.percent : model?.percent) ?? 0));
  const noText = $derived(tts.text.trim() === '');
  const played = $derived(tts.duration ? (tts.played / tts.duration) * 100 : 0);
  const voiceLabel = (voice: Voice) =>
    voice.kind === 'deep' ? t('voices.deep') : t(`voices.${voice.kind}`, { number: voice.number });
</script>

<div class="tool">
  <div class="top">
    <Segmented
      label={t('language')}
      value={tts.language}
      options={LANGUAGES.map((language) => ({ value: language, label: t(`languages.${language}`) }))}
      onchange={(language) => tts.setLanguage(language)}
    />
  </div>

  <div class="body">
    <div class="text">
      <label class="hidden" for="tts-text">{t('text')}</label>
      <!-- svelte-ignore a11y_autofocus -->
      <textarea
        id="tts-text"
        bind:value={tts.text}
        placeholder={t('placeholder')}
        spellcheck="false"
        autofocus
      ></textarea>
      <div class="text-end">
        <span class="count">{t('characters', { count: tts.text.length })}</span>
        <Button size="small" onclick={() => void tts.paste()}>{t('paste')}</Button>
      </div>
    </div>

    <div class="side">
      <div class="group">
        <span class="label">{t('voice')}</span>
        <div class="chips" role="group" aria-label={t('voice')}>
          {#each VOICES as voice (voice.id)}
            <ToggleChip
              label={voiceLabel(voice)}
              pressed={voice.id === tts.voice}
              onclick={() => (tts.voice = voice.id)}
            />
          {/each}
        </div>
      </div>
      <div class="group">
        <span class="label">{t('speed')}</span>
        <div class="chips" role="group" aria-label={t('speed')}>
          {#each SPEEDS as speed (speed)}
            <ToggleChip
              label={speedLabel(speed, locale)}
              pressed={speed === tts.speed}
              onclick={() => (tts.speed = speed)}
            />
          {/each}
        </div>
      </div>
    </div>
  </div>

  <div class="end">
    {#if model?.state === 'missing'}
      <div class="engine">
        <span class="engine-name">{t('engine')}</span>
        <span class="engine-size">{formatBytes(model.bytes)}</span>
      </div>
      <Button variant="primary" onclick={() => tts.download()}>{t('download')}</Button>
    {:else if model?.state === 'downloading' || tts.speaking}
      <div class="progress" role="status">
        <span class="progress-label">{tts.speaking ? t('making') : t('engine')}</span>
        <span
          class="bar"
          role="progressbar"
          aria-label={tts.speaking ? t('making') : t('downloading')}
          aria-valuenow={percent}
          aria-valuemin="0"
          aria-valuemax="100"><span class="fill" style:width="{percent}%"></span></span
        >
        <span class="percent">{percent} %</span>
      </div>
      <Button onclick={() => tts.cancel()}>{t('cancel')}</Button>
    {:else if model?.state === 'ready'}
      <IconButton
        path={tts.playing ? PAUSE : PLAY}
        label={tts.playing ? t('pause') : t('listen')}
        size={40}
        disabled={tts.duration === 0}
        onclick={() => tts.togglePlay()}
      />
      <span
        class="bar"
        role="progressbar"
        aria-label={t('playback')}
        aria-valuenow={Math.round(played)}
        aria-valuemin="0"
        aria-valuemax="100"
        ><span class="fill" style:width="{played}%"></span></span
      >
      <span class="time">{tts.duration ? `${clock(tts.played)} / ${clock(tts.duration)}` : clock(0)}</span>
      <Segmented
        label={t('format')}
        value={tts.format}
        options={FORMATS.map((format) => ({ value: format, label: format.toUpperCase() }))}
        onchange={(format) => (tts.format = format)}
      />
      <Button disabled={tts.duration === 0} onclick={() => tts.save()}>{t('save')}</Button>
      <Button variant="primary" disabled={noText} onclick={() => tts.generate()}>{t('generate')}</Button>
    {/if}
  </div>
</div>

<style>
  .tool {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .top {
    flex: none;
    display: flex;
  }

  .body {
    flex: 1;
    min-height: 0;
    display: flex;
    gap: 20px;
  }

  .text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    border-radius: 12px;
    background: var(--surface-sunken);
    border: 1px solid var(--border-strong);
    transition: border-color var(--motion-fast);
  }

  .text:focus-within {
    border-color: var(--accent);
  }

  .hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
  }

  textarea {
    flex: 1;
    min-height: 0;
    padding: 14px 16px;
    background: transparent;
    border: none;
    outline: none;
    resize: none;
    font: inherit;
    font-size: 15px;
    line-height: 1.55;
    color: var(--text);
  }

  textarea::placeholder {
    color: var(--text-subtle);
  }

  .text-end {
    flex: none;
    padding: 8px 8px 8px 16px;
    border-top: 1px solid var(--border);
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .count,
  .engine-size,
  .percent,
  .time {
    font-size: 12.5px;
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
  }

  .side {
    width: 248px;
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 18px;
  }

  .group {
    display: flex;
    flex-direction: column;
    gap: 8px;
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

  .end {
    flex: none;
    min-height: 56px;
    padding-top: 14px;
    border-top: 1px solid var(--border);
    display: flex;
    align-items: center;
    gap: 14px;
  }

  .engine {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .engine-name {
    font-size: 14px;
    font-weight: 600;
  }

  .progress {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 14px;
  }

  .progress-label {
    flex: none;
    font-size: 13px;
    color: var(--text-muted);
  }

  .bar {
    flex: 1;
    min-width: 0;
    display: block;
    height: 6px;
    border-radius: 3px;
    background: var(--surface-hover);
    overflow: hidden;
  }

  .fill {
    display: block;
    height: 100%;
    border-radius: 3px;
    background: var(--accent);
    transition: width var(--motion-fast) linear;
  }

  .percent {
    flex: none;
    width: 40px;
    text-align: right;
  }

  .time {
    flex: none;
  }
</style>
