<script lang="ts">
  import { Button, Icon, moduleNative, TextField, toast, type ToolProps } from '@arthex/sdk';
  import { formats, fromHsv, parseColor, toHex, toHsv, type Color } from './color';
  import ColorWheel from './ColorWheel.svelte';
  import { t } from './i18n';
  import { ICON, ID } from './identity';
  import { exactName } from './names';
  import { recent } from './recent.svelte';

  const CAN_PICK = navigator.userAgent.includes('Windows');
  const START: Color = { r: 76, g: 194, b: 255, a: 1 };

  let { input }: ToolProps = $props();

  const read = (text: string): Color | null => {
    const named = exactName(text);
    return parseColor(named ? named.hex : text);
  };

  // What a search result passed on is only where the tool starts.
  // svelte-ignore state_referenced_locally
  const first = (input ? read(input) : null) ?? START;

  let color = $state<Color>(first);
  let hsv = $state(toHsv(first));
  let entered = $state(toHex(first));
  let field: HTMLElement | undefined = $state();

  const css = $derived(`rgba(${color.r}, ${color.g}, ${color.b}, ${color.a})`);
  const rows = $derived(formats(color));

  /** From anywhere but the field: what it shows follows. */
  function show(next: Color) {
    color = next;
    entered = toHex(next);
  }

  /** The wheel and its sliders keep their hue when the colour turns black, gray or white. */
  function fromWheel(hue: number, saturation: number, value: number) {
    hsv = { h: hue, s: saturation, v: value };
    show(fromHsv(hsv, color.a));
  }

  function jumpTo(next: Color) {
    const turned = toHsv(next);
    hsv = { h: turned.s === 0 ? hsv.h : turned.h, s: turned.s, v: turned.v };
    show(next);
  }

  function oninput() {
    const next = read(entered);
    if (!next) return;
    color = next;
    const turned = toHsv(next);
    hsv = { h: turned.s === 0 ? hsv.h : turned.h, s: turned.s, v: turned.v };
  }

  async function copy(value: string) {
    await navigator.clipboard.writeText(value);
    recent.add(toHex({ ...color, a: 1 }));
    toast(t('copied', { value }));
  }

  async function takeFromScreen() {
    let hex: string | null;
    try {
      hex = await moduleNative(ID).callHidden<string | null>('pick');
    } catch {
      toast(t('takeFailed'));
      return;
    }
    const taken = hex ? parseColor(hex) : null;
    if (!hex || !taken) return;
    jumpTo(taken);
    recent.add(hex);
    requestAnimationFrame(() => field?.querySelector('input')?.focus());
  }
</script>

<div class="tool">
  <div class="picker">
    <ColorWheel
      hue={hsv.h}
      saturation={hsv.s}
      value={hsv.v}
      color={css}
      onpick={(h, s) => fromWheel(h, s, hsv.v || 1)}
    />
    <div class="sliders">
      <label>
        <span>{t('brightness')}</span>
        <input
          type="range"
          min="0"
          max="100"
          value={Math.round(hsv.v * 100)}
          oninput={(event) => fromWheel(hsv.h, hsv.s, event.currentTarget.valueAsNumber / 100)}
        />
      </label>
      <label>
        <span>{t('opacity')}</span>
        <input
          type="range"
          min="0"
          max="100"
          value={Math.round(color.a * 100)}
          oninput={(event) => show({ ...color, a: event.currentTarget.valueAsNumber / 100 })}
        />
      </label>
    </div>
    {#if recent.colors.length > 0}
      <div class="recent" role="group" aria-label={t('recent')}>
        {#each recent.colors as hex (hex)}
          <button
            type="button"
            aria-label={t('reuse', { value: hex })}
            style:background={hex}
            onclick={() => {
              const again = parseColor(hex);
              if (again) jumpTo(again);
            }}
          ></button>
        {/each}
      </div>
    {/if}
    {#if CAN_PICK}
      <Button variant="primary" size="large" onclick={takeFromScreen}>
        <Icon path={ICON} size={17} />
        {t('take')}
      </Button>
    {/if}
  </div>
  <div class="values">
    <div class="entry" bind:this={field}>
      <span class="swatch" style:background={css}></span>
      <TextField
        label={t('field')}
        hideLabel
        autofocus
        spellcheck={false}
        autocomplete="off"
        bind:value={entered}
        {oninput}
      />
    </div>
    <div class="list scroll">
      {#each rows as row (row.format)}
        <div class="row">
          <span class="format">{row.format}</span>
          <span class="value">{row.value}</span>
          <Button
            size="small"
            aria-label={t('copyValue', { value: row.value })}
            onclick={() => copy(row.value)}
          >
            {t('copyShort')}
          </Button>
        </div>
      {/each}
    </div>
  </div>
</div>

<style>
  .tool {
    flex: 1;
    min-height: 0;
    display: flex;
    gap: 28px;
  }

  .picker {
    width: 300px;
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .sliders {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 8px;
  }

  label {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 12.5px;
    color: var(--text-muted);
  }

  label span {
    width: 72px;
    flex: none;
  }

  label input {
    flex: 1;
    min-width: 0;
    accent-color: var(--accent);
  }

  .recent {
    display: flex;
    gap: 10px;
  }

  .recent button {
    width: 40px;
    height: 40px;
    border-radius: 12px;
    border: 1px solid var(--border-strong);
  }

  .picker :global(.button) {
    width: 100%;
    gap: 8px;
  }

  .values {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .entry {
    flex: none;
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .entry :global(.field) {
    flex: 1;
    min-width: 0;
  }

  .entry :global(input) {
    font-family: var(--font-mono);
  }

  .swatch {
    width: 44px;
    height: 44px;
    flex: none;
    border-radius: 12px;
    border: 1px solid var(--border-strong);
  }

  .list {
    flex: 1;
    min-height: 0;
    padding: 2px 18px;
    border-radius: 14px;
    background: var(--surface);
    border: 1px solid var(--border);
    overflow-x: hidden;
  }

  .row {
    padding: 11px 0;
    display: flex;
    align-items: center;
    gap: 14px;
  }

  .row + .row {
    border-top: 1px solid var(--border);
  }

  .format {
    width: 48px;
    flex: none;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.06em;
    color: var(--text-subtle);
  }

  .value {
    flex: 1;
    min-width: 0;
    overflow-wrap: anywhere;
    font-family: var(--font-mono);
    font-size: 14.5px;
  }
</style>
