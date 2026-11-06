<script lang="ts">
  import { Button, Modal, Segmented, TextField } from '@arthex/sdk';
  import { calendar } from './calendar.svelte';
  import { t } from './i18n';
  import {
    defaultReminder,
    isBackwards,
    itemOf,
    ITEM_TINTS,
    REMINDERS,
    type Draft,
    type ItemKind,
  } from './items';

  const NONE = 'none';

  let { initial }: { initial: Draft } = $props();

  // What it was opened on is only where the form starts.
  // svelte-ignore state_referenced_locally
  let draft = $state({ ...initial });

  const title = $derived.by(() => {
    if (initial.id) return t('editor.edit');
    return draft.kind === 'range' ? t('editor.newRange') : t('editor.newEvent');
  });
  const kinds = $derived([
    { value: 'event' as const, label: t('editor.event') },
    { value: 'range' as const, label: t('editor.range') },
  ]);
  const canSave = $derived(itemOf(draft, draft.id || 'new') !== null);

  function reminderLabel(kind: ItemKind, minutes: number | null): string {
    switch (minutes) {
      case null:
        return t('reminders.none');
      case 0:
        return t('reminders.atStart');
      case 10:
        return t('reminders.minutes10');
      case 60:
        return kind === 'range' ? t('reminders.hourEnd') : t('reminders.hour1');
      case 1440:
        return t('reminders.day1');
      default:
        return t('reminders.days3');
    }
  }

  const reminderOptions = $derived(
    REMINDERS[draft.kind].map((minutes) => ({
      value: minutes === null ? NONE : String(minutes),
      label: reminderLabel(draft.kind, minutes),
    })),
  );

  function chooseKind(kind: ItemKind) {
    draft.kind = kind;
    draft.reminder = defaultReminder(kind);
  }

  function save() {
    void calendar.save(draft);
  }

  function saveOnEnter(event: KeyboardEvent) {
    if (event.key !== 'Enter') return;
    event.preventDefault();
    if (canSave) save();
  }
</script>

<Modal {title} onclose={() => calendar.close()}>
  <div class="kind">
    <Segmented options={kinds} value={draft.kind} label={t('editor.kind')} onchange={chooseKind} />
  </div>
  <TextField
    label={t('editor.title')}
    autofocus
    maxlength={200}
    bind:value={draft.title}
    onkeydown={saveOnEnter}
  />
  {#if draft.kind === 'event'}
    <TextField label={t('editor.date')} type="date" bind:value={draft.date} />
    <div class="pair">
      <TextField label={t('editor.start')} type="time" bind:value={draft.startTime} />
      <TextField label={t('editor.end')} type="time" bind:value={draft.endTime} />
    </div>
  {:else}
    <div class="pair">
      <TextField label={t('editor.from')} type="date" bind:value={draft.from} />
      <TextField label={t('editor.at')} type="time" bind:value={draft.fromTime} />
      <TextField label={t('editor.to')} type="date" bind:value={draft.to} />
      <TextField label={t('editor.at')} type="time" bind:value={draft.toTime} />
    </div>
  {/if}
  {#if isBackwards(draft)}
    <span class="bad">{t('editor.bad')}</span>
  {/if}
  <div class="choice">
    <span>{t('editor.reminder')}</span>
    <div class="kind">
      <Segmented
        options={reminderOptions}
        value={draft.reminder === null ? NONE : String(draft.reminder)}
        label={t('editor.reminder')}
        onchange={(value) => (draft.reminder = value === NONE ? null : Number(value))}
      />
    </div>
  </div>
  <div class="choice">
    <span>{t('editor.color')}</span>
    <div class="tints" role="radiogroup" aria-label={t('editor.color')}>
      {#each ITEM_TINTS as tint (tint)}
        <button
          type="button"
          role="radio"
          class="tint"
          class:on={draft.tint === tint}
          style:background="var(--tint-{tint})"
          aria-checked={draft.tint === tint}
          aria-label={t(`tints.${tint}`)}
          onclick={() => (draft.tint = tint)}
        ></button>
      {/each}
    </div>
  </div>
  {#snippet actions()}
    {#if initial.id}
      <Button variant="quiet-danger" onclick={() => calendar.remove(initial.id)}>{t('editor.delete')}</Button>
    {/if}
    <span class="space"></span>
    <Button onclick={() => calendar.close()}>{t('editor.cancel')}</Button>
    <Button variant="primary" disabled={!canSave} onclick={save}>{t('editor.save')}</Button>
  {/snippet}
</Modal>

<style>
  .kind {
    align-self: flex-start;
  }

  .pair {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 10px 12px;
  }

  .bad {
    font-size: 12.5px;
    color: var(--danger-text);
  }

  .choice {
    display: flex;
    flex-direction: column;
    gap: 7px;
    font-size: 13px;
    color: var(--text-muted);
  }

  .tints {
    display: flex;
    gap: 10px;
  }

  .tint {
    width: 26px;
    height: 26px;
    border-radius: 50%;
  }

  .on {
    box-shadow:
      0 0 0 2px var(--surface-dialog),
      0 0 0 4px var(--text);
  }

  .space {
    flex: 1;
  }
</style>
