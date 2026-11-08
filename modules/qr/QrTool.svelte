<script lang="ts">
  import { Segmented, type ToolProps } from '@arthex/sdk';
  import { EMPTY_FIELDS, startOf, type Fields, type Kind } from './content';
  import CreateView from './CreateView.svelte';
  import { t } from './i18n';
  import { parseInput } from './input';
  import ReadView from './ReadView.svelte';

  type Tab = 'create' | 'read';

  let { input }: ToolProps = $props();

  // What a search result passed on is only where the tool starts.
  // svelte-ignore state_referenced_locally
  const start = parseInput(input);
  const first = start.tab === 'create' && start.text ? startOf(start.text) : null;

  let tab = $state<Tab>(start.tab);
  let created = $state<{ kind: Kind; fields: Fields }>({
    kind: first?.kind ?? 'link',
    fields: { ...EMPTY_FIELDS, ...first?.fields },
  });
  /** A new Create tab each time a read code is edited. */
  let edits = $state(0);

  const tabs = $derived<{ value: Tab; label: string }[]>([
    { value: 'create', label: t('tabs.create') },
    { value: 'read', label: t('tabs.read') },
  ]);

  function edit(kind: Kind, fields: Partial<Fields>) {
    created = { kind, fields: { ...EMPTY_FIELDS, ...fields } };
    edits += 1;
    tab = 'create';
  }
</script>

<div class="tool">
  <div class="top">
    <Segmented options={tabs} value={tab} label={t('mode')} onchange={(value) => (tab = value)} />
  </div>
  {#if tab === 'create'}
    {#key edits}
      <CreateView kind={created.kind} fields={created.fields} onchange={(next) => (created = next)} />
    {/key}
  {:else}
    <ReadView onedit={edit} />
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

  .top {
    flex: none;
    display: flex;
  }
</style>
