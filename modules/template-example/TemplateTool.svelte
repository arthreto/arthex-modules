<script lang="ts">
  import {
    Button,
    closeTool,
    formatBytes,
    getLocale,
    Icon,
    moduleNative,
    Segmented,
    TextField,
    toast,
    ToggleChip,
    type ToolProps,
  } from '@arthex/sdk';
  import {
    ALL_FORMATS,
    DEFAULT_CHOICES,
    FAMILIES,
    FILES,
    HEIGHTS,
    PAGES,
    PICTURES,
    ROWS,
    SECONDS,
    SIZES,
    UNITS,
    WITH_ROWS,
    cleanFormat,
    createRequest,
    familyOf,
    iconSize,
    nameProblem,
    sizeInBytes,
    type Choices,
    type Family,
    type Unit,
  } from './choices';
  import { creationStarted } from './creations';
  import { t } from './i18n';
  import { ID } from './identity';
  import type en from './locales/en.json';

  type ContentFormat = keyof typeof en.contents;
  type Duration = keyof typeof en.durations;

  const native = moduleNative(ID);

  let { input }: ToolProps = $props();

  let folder = $state('');
  let name = $state(t('defaultName'));
  let format = $state('png');
  let choices = $state<Choices>({ ...DEFAULT_CHOICES });
  let preview = $state('');
  let freeName = $state<string | null>(null);
  let creating = $state(false);

  const family = $derived(familyOf(format));
  const problem = $derived(nameProblem(name));
  const sizeBytes = $derived(sizeInBytes(choices.sizeValue, choices.unit));
  const cannotCreate = $derived(
    problem !== null ||
      format === '' ||
      folder === '' ||
      (family === 'sized' && sizeBytes === null) ||
      creating,
  );
  const wanted = $derived(`${name.trim()}.${format}`);
  const formats = $derived.by(() => {
    const known = FAMILIES.find((each) => each.id === family)?.formats ?? [];
    return family === 'sized' && format !== '' && !known.includes(format) ? [...known, format] : known;
  });
  const contents = $derived(
    family === 'document' || family === 'audio' || family === 'video' || family === 'archive'
      ? t(`contents.${format as ContentFormat}`).split('|')
      : [],
  );
  const duration = (seconds: number) => t(`durations.${String(seconds) as Duration}`);
  const numbers = $derived(new Intl.NumberFormat(getLocale()));
  const picture = $derived(PICTURES.find((each) => each.id === choices.picture) ?? PICTURES[0]);

  $effect(() => {
    if (input) folder = input;
    else void native.call<string | null>('defaultFolder').then((found) => (folder = found ?? ''));
  });

  $effect(() => {
    const asked = { format, rows: choices.rows };
    if (family !== 'text' && family !== 'code') return;
    void native
      .call<string | null>('preview', asked)
      .then((text) => (preview = text ?? ''))
      .catch(() => (preview = ''));
  });

  $effect(() => {
    const asked = { folder, name, format };
    if (folder === '' || problem !== null) {
      freeName = null;
      return;
    }
    void native
      .call<string | null>('freeName', asked)
      .then((found) => (freeName = found))
      .catch(() => (freeName = null));
  });

  function details(): string {
    switch (family) {
      case 'image': {
        const [width, height] = choices.size ?? [picture.width, picture.height];
        const [shownWidth, shownHeight] = format === 'ico' ? iconSize(width, height) : [width, height];
        return `${shownWidth} × ${shownHeight} · ${t(`pictures.${picture.id}`).toLowerCase()}`;
      }
      case 'text':
      case 'code':
        return WITH_ROWS.includes(format) ? t('summary.rows', { count: choices.rows }) : t('summary.fixed');
      case 'document':
        return t('summary.pages', { count: choices.pages });
      case 'audio':
        return duration(choices.seconds);
      case 'video':
        return `${duration(choices.seconds)} · ${choices.height === 2160 ? '4K' : `${choices.height}p`}`;
      case 'archive':
        return t('summary.files', { count: choices.files });
      case 'sized':
        return sizeBytes === null
          ? t('summary.sizeToFix')
          : `${formatBytes(sizeBytes)} · ${t(choices.random ? 'summary.random' : 'summary.zeros')}`;
    }
  }

  const summary = $derived.by(() => {
    const parts = [t(`families.${family}`), details()];
    if (freeName !== null && freeName !== wanted) parts.push(t('summary.exists', { file: wanted }));
    return parts.join(' · ');
  });

  function pickFamily(id: Family) {
    const known = FAMILIES.find((each) => each.id === id)?.formats ?? [];
    if (!known.includes(format)) format = known[0] ?? format;
  }

  async function changeFolder() {
    const picked = await native.callHidden<string | null>('pickFolder', { folder }).catch(() => null);
    if (picked) folder = picked;
  }

  async function create() {
    if (cannotCreate) return;
    creating = true;
    try {
      await native.call('create', createRequest(folder, name, format, $state.snapshot(choices)));
      creationStarted();
      closeTool();
    } catch (error) {
      toast(String(error).includes('folder') ? t('folderGone') : t('failed.other', { file: wanted }));
    }
    creating = false;
  }

  const chip = <Value,>(
    values: readonly Value[],
    label: (value: Value) => string,
    current: Value,
    set: (value: Value) => void,
  ) =>
    values.map((value) => ({
      label: label(value),
      pressed: JSON.stringify(value) === JSON.stringify(current),
      set: () => set(value),
    }));

  const options = $derived.by(() => {
    switch (family) {
      case 'image':
        return [
          {
            label: t('pixels'),
            chips: chip(
              SIZES,
              (size) => (size ? `${size[0]} × ${size[1]}` : t('original')),
              choices.size,
              (size) => (choices.size = size),
            ),
          },
        ];
      case 'text':
      case 'code':
        return WITH_ROWS.includes(format)
          ? [
              {
                label: t('rows'),
                chips: chip(
                  ROWS,
                  (rows) => numbers.format(rows),
                  choices.rows,
                  (rows) => (choices.rows = rows),
                ),
              },
            ]
          : [];
      case 'document':
        return [
          {
            label: t('pages'),
            chips: chip(PAGES, String, choices.pages, (pages) => (choices.pages = pages)),
          },
        ];
      case 'audio':
        return [
          {
            label: t('duration'),
            chips: chip(SECONDS, duration, choices.seconds, (seconds) => (choices.seconds = seconds)),
          },
        ];
      case 'video':
        return [
          {
            label: t('duration'),
            chips: chip(SECONDS, duration, choices.seconds, (seconds) => (choices.seconds = seconds)),
          },
          {
            label: t('resolution'),
            chips: chip(
              HEIGHTS,
              (height) => (height === 2160 ? '4K' : `${height}p`),
              choices.height,
              (height) => (choices.height = height),
            ),
          },
        ];
      case 'archive':
        return [
          {
            label: t('files'),
            chips: chip(FILES, String, choices.files, (files) => (choices.files = files)),
          },
        ];
      case 'sized':
        return [
          {
            label: t('contentLabel'),
            chips: chip(
              [false, true],
              (random) => t(random ? 'fill.random' : 'fill.zeros'),
              choices.random,
              (random) => (choices.random = random),
            ),
          },
        ];
    }
  });
</script>

<div class="tool">
  <div class="folder">
    <Icon path="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2Z" size={16} />
    <span class="path" title={folder} aria-label={t('folder')}>{folder}</span>
    <Button size="small" onclick={changeFolder}>{t('change')}</Button>
  </div>

  <div class="names">
    <div class="name">
      <TextField
        label={t('nameLabel')}
        bind:value={name}
        autofocus
        spellcheck="false"
        autocomplete="off"
        aria-invalid={problem !== null}
      />
    </div>
    <div class="format">
      <TextField
        label={t('formatLabel')}
        value={format}
        oninput={(event) => (format = cleanFormat(event.currentTarget.value))}
        list="template-formats"
        spellcheck="false"
        autocomplete="off"
      />
      <span class="dot" aria-hidden="true">.</span>
      <datalist id="template-formats">
        {#each ALL_FORMATS as each (each)}<option value={each}></option>{/each}
      </datalist>
    </div>
  </div>
  {#if problem}
    <span class="problem" role="alert">{t(`nameProblems.${problem}`)}</span>
  {/if}

  <div class="kinds">
    <Segmented
      label={t('familyLabel')}
      options={FAMILIES.map((each) => ({ value: each.id, label: t(`families.${each.id}`) }))}
      value={family}
      onchange={pickFamily}
    />
    <div class="chips" role="group" aria-label={t('formatsLabel')}>
      {#each formats as each (each)}
        <ToggleChip label={`.${each}`} pressed={each === format} onclick={() => (format = each)} />
      {/each}
    </div>
  </div>

  <div class="body">
    {#if family === 'image'}
      <div class="pictures" role="group" aria-label={t('pictureLabel')}>
        {#each PICTURES as each (each.id)}
          <button
            type="button"
            class="picture"
            class:on={each.id === choices.picture}
            aria-pressed={each.id === choices.picture}
            onclick={() => (choices.picture = each.id)}
          >
            <Icon path={each.icon} size={26} stroke={1.6} />
            <span>{t(`pictures.${each.id}`)}</span>
            <span class="dimensions">{each.width} × {each.height}</span>
          </button>
        {/each}
      </div>
    {:else if family === 'text' || family === 'code'}
      <section class="preview" aria-label={t('contentLabel')}><pre>{preview}</pre></section>
    {:else if family === 'sized'}
      <div class="sized">
        <TextField label={t('size')} bind:value={choices.sizeValue} inputmode="decimal" spellcheck="false" />
        <Segmented
          label={t('unit')}
          options={UNITS.map((unit) => ({ value: unit.id, label: t(`units.${unit.id}`) }))}
          value={choices.unit}
          onchange={(unit: Unit) => (choices.unit = unit)}
        />
        {#if sizeBytes === null}<span class="problem" role="alert">{t('sizeRange')}</span>{/if}
      </div>
    {:else}
      <ul class="contents" aria-label={t('contentLabel')}>
        {#each contents as line (line)}
          <li><Icon path="M5 12l5 5L20 7" size={14} stroke={2} />{line}</li>
        {/each}
      </ul>
    {/if}

    <div class="options">
      {#each options as group (group.label)}
        <div class="group">
          <span class="label">{group.label}</span>
          <div class="chips" role="group" aria-label={group.label}>
            {#each group.chips as each (each.label)}
              <ToggleChip label={each.label} pressed={each.pressed} onclick={each.set} />
            {/each}
          </div>
        </div>
      {/each}
    </div>
  </div>

  <div class="footer">
    <div class="result">
      <span class="file">{problem ? `.${format}` : (freeName ?? wanted)}</span>
      <span class="summary">{summary}</span>
    </div>
    <Button onclick={closeTool}>{t('cancel')}</Button>
    <Button variant="primary" disabled={cannotCreate} onclick={create}>{t('create')}</Button>
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

  .folder {
    flex: none;
    height: 38px;
    padding: 0 6px 0 12px;
    border-radius: 10px;
    background: var(--surface-sunken);
    border: 1px solid var(--border);
    display: flex;
    align-items: center;
    gap: 10px;
    color: var(--text-muted);
  }

  .path {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: var(--font-mono);
    font-size: 12.5px;
  }

  .names {
    flex: none;
    display: flex;
    gap: 12px;
  }

  .name {
    flex: 1;
    min-width: 0;
  }

  .format {
    position: relative;
    width: 170px;
    flex: none;
  }

  .format :global(input) {
    width: 100%;
    padding-left: 24px;
    font-family: var(--font-mono);
  }

  .dot {
    position: absolute;
    left: 13px;
    bottom: 0;
    height: 44px;
    display: flex;
    align-items: center;
    font-family: var(--font-mono);
    font-size: 15px;
    color: var(--text-subtle);
  }

  .problem {
    font-size: 12.5px;
    color: var(--danger-text);
  }

  .names + .problem {
    margin-top: -8px;
  }

  .kinds {
    flex: none;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 10px;
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .body {
    flex: 1;
    min-height: 0;
    display: flex;
    gap: 20px;
  }

  .pictures {
    flex: 1;
    min-width: 0;
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    grid-auto-rows: 1fr;
    gap: 10px;
  }

  .picture {
    border-radius: 12px;
    background: var(--surface);
    border: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    font: inherit;
    font-size: 12.5px;
    color: var(--text-muted);
    cursor: pointer;
    transition: background-color var(--motion-fast);
  }

  .picture:hover {
    background: var(--surface-hover);
    color: var(--text);
  }

  .picture.on {
    border-color: var(--accent);
    background: var(--surface-selected);
    color: var(--text);
  }

  .dimensions {
    font-size: 11.5px;
    color: var(--text-subtle);
    font-variant-numeric: tabular-nums;
  }

  .preview,
  .contents {
    flex: 1;
    min-width: 0;
    margin: 0;
    border-radius: 12px;
    background: var(--surface-sunken);
    border: 1px solid var(--border);
  }

  .preview {
    padding: 12px 16px;
    overflow: auto;
  }

  pre {
    margin: 0;
    font-family: var(--font-mono);
    font-size: 12.5px;
    line-height: 1.6;
    color: var(--text);
  }

  .contents {
    padding: 14px 18px;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 10px;
    font-size: 13px;
    color: var(--text);
  }

  .contents li {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .contents :global(svg) {
    color: var(--accent);
  }

  .sized {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 10px;
  }

  .options {
    width: 236px;
    flex: none;
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

  .footer {
    flex: none;
    padding-top: 14px;
    border-top: 1px solid var(--border);
    display: flex;
    align-items: center;
    gap: 16px;
  }

  .result {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .file {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: var(--font-mono);
    font-size: 14px;
    font-weight: 600;
  }

  .summary {
    font-size: 12.5px;
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
  }

  @media (prefers-reduced-motion: reduce) {
    .picture {
      transition: none;
    }
  }
</style>
