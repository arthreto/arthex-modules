<script lang="ts">
  import {
    Button,
    closeTool,
    moduleNative,
    Segmented,
    Skeleton,
    Switch,
    toast,
    type ToolProps,
  } from '@arthex/sdk';
  import { onMount } from 'svelte';
  import { t } from './i18n';
  import { ID, SHOW_IMAGE } from './identity';
  import { fitted, isInZone, isZone, textOf, zoneBetween, type Point, type Word, type Zone } from './words';

  interface ShownImage {
    src: string;
    width: number;
    height: number;
  }

  interface ReadLanguage {
    tag: string;
    name: string;
  }

  const native = moduleNative(ID);

  let { input }: ToolProps = $props();

  let image = $state<ShownImage | null>(null);
  let words = $state<Word[]>([]);
  let reading = $state(false);
  let problem = $state<string | null>(null);
  let languages = $state<ReadLanguage[]>([]);
  let language = $state<string | null>(null);
  let propose = $state(true);
  let zone = $state<Zone | null>(null);
  let startedAt = $state<Point | null>(null);
  let edited = $state<string | null>(null);
  let roomWidth = $state(0);
  let roomHeight = $state(0);

  const chosen = $derived(words.filter((word) => isInZone(word, zone)));
  const text = $derived(edited ?? textOf(chosen));
  const shown = $derived(image && fitted(image.width, image.height, roomWidth, roomHeight));
  const languageOptions = $derived(languages.map(({ tag, name }) => ({ value: tag, label: name })));

  const percent = (part: number, whole: number) => `${(part / whole) * 100}%`;

  async function read() {
    reading = true;
    problem = null;
    edited = null;
    try {
      words = await native.call<Word[]>('read', { language });
    } catch (error) {
      words = [];
      problem = String(error).includes('no-language') ? t('noLanguage') : t('failed');
    }
    reading = false;
  }

  async function showCurrent() {
    zone = null;
    words = [];
    reading = true;
    try {
      image = await native.call<ShownImage | null>('image');
    } catch {
      image = null;
    }
    if (image) await read();
    else reading = false;
  }

  /** The screen, or a file: only an image chosen is shown, and cancelling keeps the current one. */
  async function choose(chooser: () => Promise<boolean>): Promise<boolean> {
    let found = false;
    try {
      found = await chooser();
    } catch {
      toast(t('failed'));
    }
    if (found) await showCurrent();
    return found;
  }

  const takeFromScreen = () => choose(() => native.callHidden<boolean>('select'));
  const openImage = () => choose(() => native.call<boolean>('openImage'));

  function chooseLanguage(tag: string) {
    language = tag;
    void read();
  }

  function setPropose(next: boolean) {
    propose = next;
    native.call('setSettings', { propose: next }).catch(() => undefined);
  }

  async function copy() {
    await navigator.clipboard.writeText(text);
    toast(t('copied'));
  }

  function pointIn(event: PointerEvent): Point | null {
    if (!image) return null;
    const box = (event.currentTarget as HTMLElement).getBoundingClientRect();
    return {
      x: ((event.clientX - box.left) / box.width) * image.width,
      y: ((event.clientY - box.top) / box.height) * image.height,
    };
  }

  function onpointerdown(event: PointerEvent) {
    event.preventDefault();
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
    startedAt = pointIn(event);
    zone = null;
    edited = null;
  }

  function onpointermove(event: PointerEvent) {
    const point = pointIn(event);
    if (startedAt && point) zone = zoneBetween(startedAt, point);
  }

  function onpointerup() {
    startedAt = null;
    if (zone && !isZone(zone)) zone = null;
  }

  onMount(() => {
    const stop = native.on('open', () => void showCurrent());
    native
      .call<{ languages: ReadLanguage[]; current: string | null }>('languages')
      .then((answer) => {
        languages = answer.languages;
        language ??= answer.current;
      })
      .catch(() => undefined);
    native
      .call<{ propose: boolean }>('settings')
      .then((answer) => (propose = answer.propose))
      .catch(() => undefined);
    // From the search, the tool goes straight to the screen, as the eyedropper does.
    if (input === SHOW_IMAGE) void showCurrent();
    else
      void takeFromScreen().then((found) => {
        if (!found) closeTool();
      });
    return stop;
  });
</script>

<div class="tool">
  <div class="main">
    <div class="room" bind:clientWidth={roomWidth} bind:clientHeight={roomHeight}>
      {#if image && shown}
        <!-- A zone is traced with the mouse; the text it chooses is in the field beside it. -->
        <div
          class="image"
          role="img"
          aria-label={t('image')}
          style:width="{shown.width}px"
          style:height="{shown.height}px"
          {onpointerdown}
          {onpointermove}
          {onpointerup}
        >
          <img src={image.src} alt="" draggable="false" />
          {#each words as word, index (index)}
            <span
              class="word"
              class:chosen={zone && isInZone(word, zone)}
              class:found={!zone}
              style:left={percent(word.x, image.width)}
              style:top={percent(word.y, image.height)}
              style:width={percent(word.w, image.width)}
              style:height={percent(word.h, image.height)}
            ></span>
          {/each}
          {#if zone}
            <span
              class="zone"
              style:left={percent(zone.x, image.width)}
              style:top={percent(zone.y, image.height)}
              style:width={percent(zone.w, image.width)}
              style:height={percent(zone.h, image.height)}
            ></span>
          {/if}
        </div>
      {/if}
    </div>
    <div class="side">
      <div class="head">
        <h3>{t('text')}</h3>
        {#if languageOptions.length > 1 && language}
          <Segmented
            options={languageOptions}
            value={language}
            label={t('language')}
            onchange={chooseLanguage}
          />
        {/if}
      </div>
      {#if reading}
        <div class="field loading">
          <Skeleton width="92%" height="13px" />
          <Skeleton width="78%" height="13px" />
          <Skeleton width="85%" height="13px" />
        </div>
      {:else}
        <textarea
          class="field scroll"
          aria-label={t('textField')}
          placeholder={problem ?? t('noText')}
          spellcheck="false"
          value={text}
          oninput={(event) => (edited = event.currentTarget.value)}></textarea>
      {/if}
      <Button variant="primary" size="large" disabled={text.trim() === ''} onclick={copy}>{t('copy')}</Button>
    </div>
  </div>
  <div class="actions">
    <Button onclick={openImage}>{t('openImage')}</Button>
    <Button onclick={takeFromScreen}>{t('takeFromScreen')}</Button>
    <span class="propose">{t('propose')}</span>
    <Switch label={t('propose')} checked={propose} onchange={setPropose} />
  </div>
</div>

<style>
  .tool {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .main {
    flex: 1;
    min-height: 0;
    display: flex;
    gap: 24px;
  }

  .room {
    flex: 1;
    min-width: 0;
    border-radius: 14px;
    background: var(--surface-sunken);
    border: 1px solid var(--border);
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
  }

  .image {
    position: relative;
    flex: none;
    overflow: hidden;
    cursor: crosshair;
    user-select: none;
    touch-action: none;
  }

  img {
    display: block;
    width: 100%;
    height: 100%;
  }

  .word,
  .zone {
    position: absolute;
    pointer-events: none;
  }

  .word {
    border-radius: 3px;
  }

  .word.found {
    background: color-mix(in srgb, var(--accent) 16%, transparent);
  }

  .word.chosen {
    background: color-mix(in srgb, var(--accent) 38%, transparent);
  }

  .zone {
    box-sizing: border-box;
    border: 2px solid var(--accent);
    border-radius: 6px;
  }

  .side {
    width: 280px;
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .side :global(.button) {
    width: 100%;
  }

  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }

  h3 {
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-subtle);
  }

  .field {
    flex: 1;
    min-height: 0;
    box-sizing: border-box;
    padding: 12px 14px;
    border-radius: 10px;
    background: var(--surface-sunken);
    border: 1px solid var(--border-strong);
    font: inherit;
    font-size: 14px;
    line-height: 1.5;
    color: var(--text);
    resize: none;
  }

  .field::placeholder {
    color: var(--text-subtle);
  }

  .loading {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .actions {
    flex: none;
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .propose {
    margin-left: auto;
    font-size: 13px;
    color: var(--text-muted);
  }
</style>
