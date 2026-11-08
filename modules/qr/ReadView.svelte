<script lang="ts">
  import { Button, getLocale, hideOverlay, Icon, moduleNative, Skeleton, toast } from '@arthex/sdk';
  import { analyze, type Action, type FactValue } from './analysis';
  import type { Fields, Kind } from './content';
  import { t } from './i18n';
  import { CHECK, ICON, ID, WARNING } from './identity';
  import { KIND_ICONS } from './kinds';
  import { calendarOf, dayOf } from './calendar';
  import { imageIn, read, type Reading } from './reading';

  let { onedit }: { onedit: (kind: Kind, fields: Partial<Fields>) => void } = $props();

  const native = moduleNative(ID);

  let reading = $state<Reading | null>(null);
  let busy = $state(false);
  let reveal = $state(false);
  let picker = $state<HTMLInputElement>();
  let roomWidth = $state(0);
  let roomHeight = $state(0);

  const found = $derived(reading?.found ?? null);
  const analysis = $derived(found ? analyze(found.content) : null);
  const locale = $derived(getLocale());
  const shown = $derived.by(() => {
    if (!reading || roomWidth === 0 || roomHeight === 0) return null;
    const { width, height } = reading.shot;
    const scale = Math.min(roomWidth / width, roomHeight / height);
    return { width: Math.floor(width * scale), height: Math.floor(height * scale) };
  });
  const outline = $derived(found?.corners.map((corner) => `${corner.x},${corner.y}`).join(' ') ?? '');

  async function show(source: Blob | string) {
    busy = true;
    reveal = false;
    try {
      reading = await read(source);
    } catch {
      toast(t('failed'));
    }
    busy = false;
  }

  async function capture() {
    try {
      const src = await native.callHidden<string | null>('select');
      if (src) await show(src);
    } catch {
      toast(t('failed'));
    }
  }

  function picked(event: Event & { currentTarget: HTMLInputElement }) {
    const image = imageIn(event.currentTarget.files);
    event.currentTarget.value = '';
    if (image) void show(image);
  }

  function dropped(event: DragEvent) {
    event.preventDefault();
    const image = imageIn(event.dataTransfer?.files);
    if (image) void show(image);
  }

  function pasted(event: ClipboardEvent) {
    const image = imageIn(event.clipboardData?.files);
    if (!image) return;
    event.preventDefault();
    void show(image);
  }

  function valueOf(value: FactValue, secret = false): string {
    if ('text' in value)
      return secret && !reveal ? '•'.repeat(Math.min(12, [...value.text].length)) : value.text;
    if ('day' in value) return dayOf(value.day, locale);
    return t(`values.${value.message}`, value.values);
  }

  async function run(action: Action) {
    try {
      switch (action.id) {
        case 'connect': {
          const network = analysis?.network;
          if (!network) return;
          toast(t('connecting', { name: network.ssid }));
          await native
            .call('connect', network)
            .catch(() => toast(t('connectFailed', { name: network.ssid })));
          return;
        }
        case 'openLink':
        case 'writeEmail':
        case 'call':
        case 'openMap':
          await native.call('open', { address: action.value });
          hideOverlay();
          return;
        case 'addContact':
          await native.call('openFile', { kind: 'contact', content: action.value });
          hideOverlay();
          return;
        case 'addEvent':
          await native.call('openFile', { kind: 'event', content: calendarOf(action.value) });
          hideOverlay();
          return;
        default:
          await navigator.clipboard.writeText(action.value);
          toast(t('copied'));
      }
    } catch {
      toast(t('failed'));
    }
  }
</script>

<svelte:window onpaste={pasted} />

<input
  bind:this={picker}
  class="picker"
  type="file"
  accept="image/*"
  tabindex="-1"
  aria-hidden="true"
  onchange={picked}
/>

{#if busy}
  <div class="view" aria-busy="true" aria-label={t('reading')}>
    <div class="side"><Skeleton width="100%" height="100%" radius="14px" /></div>
    <div class="result">
      <Skeleton width="40%" height="22px" />
      <Skeleton width="100%" height="58px" radius="10px" />
      <Skeleton width="80%" height="16px" />
      <Skeleton width="65%" height="16px" />
      <Skeleton width="72%" height="16px" />
    </div>
  </div>
{:else if !reading}
  <div
    class="drop"
    role="region"
    aria-label={t('tabs.read')}
    ondragover={(event) => event.preventDefault()}
    ondrop={dropped}
  >
    <span class="symbol"><Icon path={ICON} size={44} stroke={1.4} /></span>
    <div class="buttons">
      <Button variant="primary" onclick={capture}>{t('capture')}</Button>
      <Button onclick={() => picker?.click()}>{t('chooseImage')}</Button>
    </div>
    <span class="hint">{t('dropHint')}</span>
  </div>
{:else}
  <div
    class="view fade-in"
    role="region"
    aria-label={t('tabs.read')}
    ondragover={(event) => event.preventDefault()}
    ondrop={dropped}
  >
    <div class="side">
      <div class="room" bind:clientWidth={roomWidth} bind:clientHeight={roomHeight}>
        {#if shown}
          <div class="shot" style:width="{shown.width}px" style:height="{shown.height}px">
            <img src={reading.shot.src} alt={t('shot')} />
            {#if found}
              <svg
                viewBox="0 0 {reading.shot.width} {reading.shot.height}"
                preserveAspectRatio="none"
                aria-hidden="true"
              >
                <polygon points={outline} stroke-width={Math.max(2, reading.shot.width / 160)} />
              </svg>
            {/if}
          </div>
        {/if}
      </div>
      <div class="buttons">
        <Button onclick={capture}>{t('capture')}</Button>
        <Button onclick={() => picker?.click()}>{t('chooseImage')}</Button>
      </div>
    </div>

    {#if found && analysis}
      <div class="result">
        <div class="heading">
          <span class="tile"><Icon path={KIND_ICONS[analysis.kind]} size={17} /></span>
          <h3>{t(`readKinds.${analysis.kind}`)}</h3>
        </div>
        <div class="content scroll">{found.content}</div>
        <section class="facts scroll" aria-label={t('analysis')}>
          {#each analysis.facts as fact, index (index)}
            <div class="fact">
              <span class="label">{t(`facts.${fact.label}`)}</span>
              <span class="value" class:ok={fact.tone === 'ok'} class:warn={fact.tone === 'warn'}>
                {#if fact.tone === 'ok'}
                  <Icon path={CHECK} size={15} stroke={2.4} />
                {:else if fact.tone === 'warn'}
                  <Icon path={WARNING} size={15} stroke={2} />
                {/if}
                <span class:mono={fact.mono || fact.secret}>{valueOf(fact.value, fact.secret)}</span>
              </span>
              {#if fact.secret}
                <Button size="small" onclick={() => (reveal = !reveal)}
                  >{reveal ? t('hide') : t('show')}</Button
                >
              {/if}
            </div>
          {/each}
          <h4>{t('codeSection')}</h4>
          <div class="fact">
            <span class="label">{t('version')}</span>
            <span class="value"
              >{t('versionValue', { version: found.version, size: found.version * 4 + 17 })}</span
            >
          </div>
          <div class="fact">
            <span class="label">{t('data')}</span>
            <span class="value">{t('bytes', { count: found.bytes })}</span>
          </div>
        </section>
        <div class="actions">
          {#if analysis.edit}
            {@const edit = analysis.edit}
            <Button onclick={() => onedit(edit.kind, edit.fields)}>{t('edit')}</Button>
          {/if}
          {#each analysis.actions as action (action.id)}
            <Button variant={action.primary ? 'primary' : 'default'} onclick={() => run(action)}>
              {t(`actions.${action.id}`)}
            </Button>
          {/each}
        </div>
      </div>
    {:else}
      <div class="nothing" role="status">
        <span class="title">{t('noCode')}</span>
        <span class="hint">{t('noCodeHint')}</span>
      </div>
    {/if}
  </div>
{/if}

<style>
  .picker {
    display: none;
  }

  .view {
    flex: 1;
    min-height: 0;
    display: flex;
    gap: 28px;
  }

  .drop {
    flex: 1;
    min-height: 0;
    border-radius: 14px;
    background: var(--surface);
    border: 1px dashed var(--border-strong);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 18px;
  }

  .symbol {
    display: flex;
    color: var(--text-muted);
  }

  .buttons {
    display: flex;
    gap: 10px;
  }

  .hint {
    font-size: 12.5px;
    color: var(--text-subtle);
  }

  .side {
    width: 320px;
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .side .buttons {
    gap: 8px;
  }

  .side .buttons > :global(*) {
    flex: 1;
  }

  .room {
    flex: 1;
    min-height: 0;
    padding: 12px;
    border-radius: 14px;
    background: var(--surface-sunken);
    border: 1px solid var(--border);
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
    box-sizing: border-box;
  }

  .shot {
    position: relative;
    flex: none;
  }

  img,
  svg {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    border-radius: 6px;
  }

  polygon {
    fill: none;
    stroke: var(--success);
    stroke-linejoin: round;
  }

  .result {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .heading {
    flex: none;
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .tile {
    width: 34px;
    height: 34px;
    flex: none;
    border-radius: 10px;
    background: var(--surface-raised);
    border: 1px solid var(--border-strong);
    color: var(--tint-amber);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  h3 {
    margin: 0;
    font-size: 17px;
    font-weight: 600;
  }

  .content {
    flex: none;
    max-height: 84px;
    padding: 10px 12px;
    border-radius: 10px;
    background: var(--surface-sunken);
    border: 1px solid var(--border-strong);
    font-family: var(--font-mono);
    font-size: 12.5px;
    line-height: 1.5;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    overflow-y: auto;
    user-select: text;
  }

  .facts {
    flex: 1;
    min-height: 0;
    padding: 4px 14px;
    border-radius: 14px;
    background: var(--surface);
    border: 1px solid var(--border);
    overflow-x: hidden;
    overflow-y: auto;
  }

  .fact {
    padding: 8px 0;
    display: flex;
    align-items: flex-start;
    gap: 12px;
    font-size: 13px;
  }

  .fact + .fact {
    border-top: 1px solid var(--border);
  }

  .label {
    width: 120px;
    flex: none;
    color: var(--text-subtle);
  }

  .value {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: flex-start;
    gap: 7px;
    overflow-wrap: anywhere;
  }

  .value > :global(svg) {
    flex: none;
    margin-top: 2px;
  }

  .ok {
    color: var(--success);
  }

  .warn {
    color: var(--warning);
  }

  .mono {
    font-family: var(--font-mono);
  }

  h4 {
    margin: 0;
    padding: 12px 0 2px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-subtle);
  }

  .actions {
    flex: none;
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 8px;
  }

  .nothing {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    text-align: center;
  }

  .title {
    font-size: 15px;
    font-weight: 600;
  }
</style>
