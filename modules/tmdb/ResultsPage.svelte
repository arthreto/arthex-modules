<script lang="ts">
  import { Segmented, Skeleton } from '@arthex/sdk';
  import CardGrid from './CardGrid.svelte';
  import { t } from './i18n';
  import { problemOf, search, type Problem } from './library';
  import type { Card, MediaKind } from './media';
  import ProblemState from './ProblemState.svelte';

  interface Props {
    query: string;
    onopen: (card: Card) => void;
    onproblem: (problem: Problem) => void;
  }

  type Filter = MediaKind | 'all';

  let { query, onopen, onproblem }: Props = $props();

  let cards = $state<Card[] | null>(null);
  let problem = $state<Problem | null>(null);
  let filter = $state<Filter>('all');

  const shown = $derived((cards ?? []).filter((card) => filter === 'all' || card.kind === filter));
  const filters = $derived<{ value: Filter; label: string }[]>([
    { value: 'all', label: t('filters.all') },
    { value: 'movie', label: t('filters.movie') },
    { value: 'tv', label: t('filters.tv') },
    { value: 'person', label: t('filters.person') },
  ]);

  function load() {
    const asked = query;
    problem = null;
    cards = null;
    search(asked).then(
      (found) => {
        if (asked === query) cards = found;
      },
      (error) => {
        problem = problemOf(error);
        onproblem(problem);
      },
    );
  }

  $effect(load);
</script>

<div class="heading">
  <h3>{t('resultsFor', { query })}</h3>
  <Segmented options={filters} value={filter} label={t('filter')} onchange={(value) => (filter = value)} />
</div>
{#if problem}
  <ProblemState {problem} onretry={load} />
{:else}
  <div class="page scroll">
    {#if !cards}
      <div class="loading" aria-busy="true">
        {#each Array.from({ length: 6 }, (_, index) => index) as index (index)}
          <Skeleton width="100%" height="180px" radius="8px" />
        {/each}
      </div>
    {:else if shown.length > 0}
      <CardGrid cards={shown} {onopen} />
    {:else}
      <p class="none" role="status">{t('noResults', { query })}</p>
    {/if}
  </div>
{/if}

<style>
  .heading {
    flex: none;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  h3 {
    margin: 0;
    min-width: 0;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-subtle);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .page {
    flex: 1;
    min-height: 0;
    overflow-x: hidden;
    overflow-y: auto;
    padding-right: 4px;
  }

  .loading {
    display: grid;
    grid-template-columns: repeat(6, minmax(0, 1fr));
    gap: 22px 16px;
    padding: 6px;
  }

  .none {
    height: 100%;
    margin: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 14px;
    color: var(--text-subtle);
  }
</style>
