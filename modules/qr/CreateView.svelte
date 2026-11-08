<script lang="ts">
  import {
    Button,
    getLocale,
    Icon,
    moduleNative,
    Segmented,
    Switch,
    TextField,
    toast,
    ToggleChip,
  } from '@arthex/sdk';
  import {
    CAPACITY,
    CHECKER,
    copyImage,
    encode,
    LEVELS,
    PAPER,
    pngOf,
    svgOf,
    viewBoxOf,
    type Level,
  } from './code';
  import { contentOf, fileNameOf, KINDS, SECURITIES, type Fields, type Kind } from './content';
  import { t } from './i18n';
  import { ICON, ID } from './identity';

  type Format = 'png' | 'svg';

  interface Props {
    kind: Kind;
    fields: Fields;
    onchange: (created: { kind: Kind; fields: Fields }) => void;
  }

  const COLORS = [
    { hex: '#000000', name: 'black' },
    { hex: '#1f2937', name: 'slate' },
    { hex: '#1d4ed8', name: 'blue' },
    { hex: '#047857', name: 'green' },
    { hex: '#6d28d9', name: 'violet' },
    { hex: '#9f1239', name: 'burgundy' },
  ] as const;
  const SIZES = [512, 1024, 2048] as const;

  const native = moduleNative(ID);

  let { kind: startKind, fields: startFields, onchange }: Props = $props();

  // svelte-ignore state_referenced_locally
  let kind = $state<Kind>(startKind);
  // svelte-ignore state_referenced_locally
  let fields = $state<Fields>({ ...startFields });
  let level = $state<Level>('M');
  let color = $state<string>(COLORS[0].hex);
  let transparent = $state(false);
  let pixels = $state<number>(1024);
  let format = $state<Format>('png');

  const content = $derived(contentOf(kind, fields));
  const code = $derived(encode(content, level));
  const bytes = $derived(new TextEncoder().encode(content).length);
  const paper = $derived(transparent ? null : PAPER);
  const locale = $derived(getLocale());

  $effect(() => onchange({ kind, fields: { ...fields } }));

  const levels = LEVELS.map((value) => ({ value, label: value }));

  async function copy() {
    if (!code) return;
    try {
      await copyImage(pngOf(code, color, paper, pixels));
      toast(t('imageCopied'));
    } catch {
      toast(t('failed'));
    }
  }

  function base64Of(blob: Blob): Promise<string> {
    return new Promise((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = () => resolve(String(reader.result).replace(/^data:[^,]*,/, ''));
      reader.onerror = () => reject(reader.error ?? new Error('unreadable'));
      reader.readAsDataURL(blob);
    });
  }

  async function save() {
    if (!code) return;
    const name = `${fileNameOf(kind, fields, content)}.${format}`;
    try {
      const file =
        format === 'svg'
          ? { name, text: svgOf(code, color, paper, pixels) }
          : { name, base64: await base64Of(await pngOf(code, color, paper, pixels)) };
      const saved = await native.call<string | null>('save', file);
      if (saved) toast(t('saved', { name: saved }));
    } catch {
      toast(t('failed'));
    }
  }
</script>

<div class="view">
  <div class="form scroll">
    <div class="chips" role="group" aria-label={t('type')}>
      {#each KINDS as value (value)}
        <ToggleChip label={t(`kinds.${value}`)} pressed={kind === value} onclick={() => (kind = value)} />
      {/each}
    </div>
    <div class="fields">
      {#if kind === 'link'}
        <TextField
          label={t('fields.address')}
          bind:value={fields.link}
          placeholder="https://"
          inputmode="url"
          spellcheck="false"
          autofocus
        />
      {:else if kind === 'text'}
        <label class="area">
          <span>{t('fields.text')}</span>
          <textarea
            class="scroll tall"
            bind:value={fields.text}
            placeholder={t('fields.textPlaceholder')}
            spellcheck="false"></textarea>
        </label>
      {:else if kind === 'wifi'}
        <TextField label={t('fields.network')} bind:value={fields.ssid} spellcheck="false" autofocus />
        <div class="choice">
          <span>{t('fields.security')}</span>
          <div class="chips" role="group" aria-label={t('fields.security')}>
            {#each SECURITIES as value (value)}
              <ToggleChip
                label={t(`securities.${value}`)}
                pressed={fields.security === value}
                onclick={() => (fields.security = value)}
              />
            {/each}
          </div>
        </div>
        {#if fields.security !== 'nopass'}
          <TextField
            label={t('fields.password')}
            bind:value={fields.password}
            spellcheck="false"
            autocomplete="off"
          />
        {/if}
        <div class="switch">
          <span>{t('fields.hidden')}</span>
          <Switch
            label={t('fields.hidden')}
            checked={fields.hidden}
            onchange={(checked) => (fields.hidden = checked)}
          />
        </div>
      {:else if kind === 'contact'}
        <div class="pair">
          <TextField label={t('fields.firstName')} bind:value={fields.firstName} autofocus />
          <TextField label={t('fields.lastName')} bind:value={fields.lastName} />
        </div>
        <TextField label={t('fields.phone')} bind:value={fields.contactPhone} inputmode="tel" />
        <TextField
          label={t('fields.email')}
          bind:value={fields.contactEmail}
          inputmode="email"
          spellcheck="false"
        />
        <TextField label={t('fields.organization')} bind:value={fields.organization} />
      {:else if kind === 'email'}
        <TextField
          label={t('fields.to')}
          bind:value={fields.emailTo}
          inputmode="email"
          spellcheck="false"
          autofocus
        />
        <TextField label={t('fields.subject')} bind:value={fields.emailSubject} />
        <label class="area">
          <span>{t('fields.message')}</span>
          <textarea class="scroll" bind:value={fields.emailBody}></textarea>
        </label>
      {:else if kind === 'phone'}
        <TextField label={t('fields.number')} bind:value={fields.phone} inputmode="tel" autofocus />
      {:else if kind === 'sms'}
        <TextField label={t('fields.number')} bind:value={fields.smsNumber} inputmode="tel" autofocus />
        <label class="area">
          <span>{t('fields.message')}</span>
          <textarea class="scroll" bind:value={fields.smsMessage}></textarea>
        </label>
      {:else}
        <div class="pair">
          <TextField
            label={t('fields.latitude')}
            bind:value={fields.latitude}
            inputmode="decimal"
            autofocus
          />
          <TextField label={t('fields.longitude')} bind:value={fields.longitude} inputmode="decimal" />
        </div>
      {/if}
    </div>
  </div>

  <div class="preview">
    {#if code}
      <div class="paper" class:transparent style:--paper={PAPER} style:--checker={CHECKER}>
        <svg
          role="img"
          aria-label={t('codeOf', { content })}
          width="244"
          height="244"
          viewBox={viewBoxOf(code)}
          shape-rendering="crispEdges"
        >
          <path d={code.path} fill={color} />
        </svg>
      </div>
      <span class="info">{t('info', { version: code.version, size: code.size, bytes: code.bytes })}</span>
    {:else if content}
      <div class="too-long" role="status">
        <span class="problem">{t('tooLong')}</span>
        <span class="info">{t('tooLongDetail', { bytes, max: CAPACITY[level].toLocaleString(locale) })}</span>
      </div>
    {:else}
      <span class="empty"><Icon path={ICON} size={56} stroke={1.4} /></span>
    {/if}
  </div>

  <div class="options">
    <div class="option">
      <span>{t('correction')}</span>
      <Segmented
        options={levels}
        value={level}
        label={t('correction')}
        onchange={(value) => (level = value)}
      />
    </div>
    <div class="option">
      <span>{t('color')}</span>
      <div class="swatches" role="group" aria-label={t('color')}>
        {#each COLORS as swatch (swatch.hex)}
          <button
            type="button"
            class="swatch"
            class:on={color === swatch.hex}
            style:background={swatch.hex}
            aria-pressed={color === swatch.hex}
            aria-label={t(`colors.${swatch.name}`)}
            title={t(`colors.${swatch.name}`)}
            onclick={() => (color = swatch.hex)}
          ></button>
        {/each}
      </div>
    </div>
    <div class="option">
      <span>{t('background')}</span>
      <div class="chips" role="group" aria-label={t('background')}>
        <ToggleChip
          label={t('backgrounds.white')}
          pressed={!transparent}
          onclick={() => (transparent = false)}
        />
        <ToggleChip
          label={t('backgrounds.transparent')}
          pressed={transparent}
          onclick={() => (transparent = true)}
        />
      </div>
    </div>
    <div class="option">
      <span>{t('pixels')}</span>
      <div class="chips" role="group" aria-label={t('pixels')}>
        {#each SIZES as size (size)}
          <ToggleChip
            label={size.toLocaleString(locale)}
            pressed={pixels === size}
            onclick={() => (pixels = size)}
          />
        {/each}
      </div>
    </div>
    <div class="option">
      <span>{t('format')}</span>
      <div class="chips" role="group" aria-label={t('format')}>
        <ToggleChip label="PNG" pressed={format === 'png'} onclick={() => (format = 'png')} />
        <ToggleChip label="SVG" pressed={format === 'svg'} onclick={() => (format = 'svg')} />
      </div>
    </div>
    <div class="buttons">
      <Button disabled={!code} onclick={copy}>{t('copyImage')}</Button>
      <Button variant="primary" disabled={!code} onclick={save}>{t('save')}</Button>
    </div>
  </div>
</div>

<style>
  .view {
    flex: 1;
    min-height: 0;
    display: flex;
    gap: 24px;
  }

  .form {
    width: 280px;
    flex: none;
    padding: 2px;
    overflow-x: hidden;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .fields {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .pair {
    display: flex;
    gap: 12px;
  }

  .pair > :global(*) {
    flex: 1;
    min-width: 0;
  }

  .choice,
  .option {
    display: flex;
    flex-direction: column;
    gap: 7px;
    font-size: 13px;
    color: var(--text-muted);
  }

  .switch {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    font-size: 13px;
    color: var(--text-muted);
  }

  .area {
    display: flex;
    flex-direction: column;
    gap: 7px;
    font-size: 13px;
    color: var(--text-muted);
  }

  textarea {
    height: 120px;
    padding: 10px 14px;
    border-radius: 10px;
    background: var(--surface-sunken);
    border: 1px solid var(--border-strong);
    font: inherit;
    font-size: 14px;
    line-height: 1.45;
    color: var(--text);
    resize: none;
    outline: none;
    transition: border-color var(--motion-fast);
  }

  .tall {
    height: 220px;
  }

  textarea:focus {
    border-color: var(--accent);
  }

  textarea::placeholder {
    color: var(--text-subtle);
  }

  .preview {
    flex: 1;
    min-width: 0;
    border-radius: 14px;
    background: var(--surface-sunken);
    border: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 14px;
  }

  .paper {
    width: 244px;
    height: 244px;
    flex: none;
    border-radius: 10px;
    overflow: hidden;
    background: var(--paper);
  }

  .transparent {
    background: repeating-conic-gradient(var(--checker) 0% 25%, var(--paper) 0% 50%) 0 0 / 16px 16px;
  }

  svg {
    display: block;
  }

  .info {
    font-size: 12.5px;
    color: var(--text-subtle);
    font-variant-numeric: tabular-nums;
    text-align: center;
  }

  .too-long {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
  }

  .problem {
    font-size: 14px;
    font-weight: 600;
    color: var(--danger-text);
  }

  .empty {
    display: flex;
    color: var(--text-subtle);
    opacity: 0.6;
  }

  .options {
    width: 212px;
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .options :global([role='radiogroup']) {
    display: flex;
  }

  .options :global([role='radiogroup'] button) {
    flex: 1;
  }

  .swatches {
    display: flex;
    gap: 8px;
  }

  .swatch {
    width: 26px;
    height: 26px;
    flex: none;
    padding: 0;
    border-radius: 50%;
    border: 2px solid var(--border-strong);
    cursor: pointer;
  }

  .swatch.on {
    box-shadow:
      0 0 0 2px var(--surface-dialog),
      0 0 0 4px var(--accent);
  }

  .swatch:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 3px;
  }

  .buttons {
    margin-top: auto;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
</style>
