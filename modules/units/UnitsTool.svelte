<script lang="ts">
  import { Button, getLocale, Panel, toast, ToggleChip, type ToolProps } from '@arthex/sdk';
  import { t, withSymbol } from './i18n';
  import { parseNumber, parseQuery } from './parse';
  import {
    CATEGORIES,
    categoryOf,
    convert,
    formatValue,
    startingUnit,
    type Category,
    type Unit,
  } from './units';

  let { input }: ToolProps = $props();

  // What a search result passed on is only where the converter starts.
  // svelte-ignore state_referenced_locally
  const start = input ? parseQuery(input) : null;
  const firstCategory: Category = start ? categoryOf(start.from) : CATEGORIES[0];

  let category = $state<Category>(firstCategory);
  let from = $state<Unit>(start?.from ?? startingUnit(firstCategory));
  let entered = $state(start ? formatValue(start.value, getLocale(), false) : '1');

  const value = $derived(parseNumber(entered));
  const units = $derived<readonly Unit[]>(category.units);
  const columns = $derived(units.length <= 3 ? 1 : units.length <= 4 ? 2 : 3);
  const rows = $derived(
    units
      .filter((unit) => unit.id !== from.id)
      .map((unit) => {
        const result = value === null ? null : convert(value, from, unit);
        return {
          unit,
          result,
          shown: result === null ? '—' : withSymbol(formatValue(result, getLocale()), unit),
        };
      }),
  );

  function pick(next: Category) {
    category = next;
    from = startingUnit(next);
  }

  async function copy(unit: Unit, result: number | null, shown: string) {
    if (result === null) return;
    await navigator.clipboard.writeText(withSymbol(formatValue(result, getLocale(), false), unit));
    toast(t('copied', { value: shown }));
  }

  function focusOnMount(node: HTMLInputElement) {
    node.focus();
    node.select();
  }
</script>

<div class="converter">
  <div class="categories" role="group" aria-label={t('category')}>
    {#each CATEGORIES as option (option.id)}
      <ToggleChip
        label={t(`categories.${option.id}`)}
        pressed={option.id === category.id}
        onclick={() => pick(option)}
      />
    {/each}
  </div>
  <div class="columns">
    <div class="pad">
      <div class="screen">
        <input
          bind:value={entered}
          use:focusOnMount
          aria-label={t('value')}
          spellcheck="false"
          autocomplete="off"
        />
        <span class="unit-name">{t(`units.${from.id}.name`)}</span>
      </div>
      <div class="keys" role="group" aria-label={t('fromUnit')} style:--columns={columns}>
        {#each units as unit (unit.id)}
          <Button
            size="large"
            variant={unit.id === from.id ? 'primary' : 'default'}
            aria-label={t(`units.${unit.id}.name`)}
            aria-pressed={unit.id === from.id}
            onclick={() => (from = unit)}>{t(`units.${unit.id}.symbol`)}</Button
          >
        {/each}
      </div>
    </div>
    <div class="side">
      <h3>{t('conversions')}</h3>
      <div class="list scroll">
        <Panel list>
          {#each rows as row (row.unit.id)}
            <button
              class="row"
              aria-label={t('copyValue', { value: row.shown })}
              onclick={() => copy(row.unit, row.result, row.shown)}
            >
              <span class="name">{t(`units.${row.unit.id}.name`)}</span>
              <span class="result">{row.shown}</span>
            </button>
          {/each}
        </Panel>
      </div>
    </div>
  </div>
</div>

<style>
  .converter {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .categories {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .columns {
    flex: 1;
    min-height: 0;
    display: flex;
    gap: 28px;
  }

  .pad {
    width: 360px;
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .screen {
    padding: 14px 20px 16px;
    border-radius: var(--radius-l);
    background: var(--surface);
    border: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 4px;
  }

  input {
    width: 100%;
    background: none;
    border: none;
    outline: none;
    text-align: right;
    font-size: 40px;
    font-weight: 600;
    letter-spacing: -0.01em;
    color: var(--text);
    font-variant-numeric: tabular-nums;
  }

  .unit-name {
    font-size: 14px;
    color: var(--text-muted);
  }

  .keys {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: repeat(var(--columns), minmax(0, 1fr));
    grid-auto-rows: 1fr;
    gap: 8px;
  }

  .keys :global(.button) {
    width: 100%;
    height: 100%;
    font-size: 16px;
  }

  .side {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  h3 {
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-subtle);
  }

  .list {
    flex: 1;
    min-height: 0;
    overflow-x: hidden;
  }

  .list :global(.panel) {
    min-height: 100%;
  }

  .row {
    width: calc(100% + 16px);
    margin-left: -8px;
    padding: 12px 8px;
    border-radius: var(--radius-s);
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }

  .row + .row {
    border-top: 1px solid var(--border);
  }

  .row:hover {
    background: var(--surface-hover);
  }

  .name {
    flex: none;
    font-size: 13px;
    color: var(--text-subtle);
  }

  .result {
    min-width: 0;
    overflow-wrap: anywhere;
    text-align: right;
    font-size: 17px;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }
</style>
