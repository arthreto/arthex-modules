<script lang="ts">
  import { Skeleton } from '@arthex/sdk';
  import CardGrid from './CardGrid.svelte';
  import { t } from './i18n';
  import { problemOf, trending, type Problem } from './library';
  import type { Card } from './media';
  import ProblemState from './ProblemState.svelte';

  interface Props {
    onopen: (card: Card) => void;
    onproblem: (problem: Problem) => void;
  }

  let { onopen, onproblem }: Props = $props();

  let cards = $state<Card[] | null>(null);
  let problem = $state<Problem | null>(null);

  function load() {
    problem = null;
    trending().then(
      (found) => (cards = found.slice(0, 18)),
      (error) => {
        problem = problemOf(error);
        onproblem(problem);
      },
    );
  }

  $effect(load);
</script>

{#if problem}
  <ProblemState {problem} onretry={load} />
{:else}
  <div class="page scroll">
    <h3>{t('trending')}</h3>
    {#if cards}
      <CardGrid {cards} {onopen} />
    {:else}
      <div class="loading" aria-busy="true">
        {#each Array.from({ length: 12 }, (_, index) => index) as index (index)}
          <Skeleton width="100%" height="180px" radius="8px" />
        {/each}
      </div>
    {/if}
  </div>
{/if}

<style>
  .page {
    flex: 1;
    min-height: 0;
    overflow-x: hidden;
    overflow-y: auto;
    padding-right: 4px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  h3 {
    margin: 0;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-subtle);
  }

  .loading {
    display: grid;
    grid-template-columns: repeat(6, minmax(0, 1fr));
    gap: 22px 16px;
    padding: 6px;
  }
</style>
