<script lang="ts">
  import { getLocale } from '@arthex/sdk';
  import type { Card } from './media';
  import { pictures } from './pictures.svelte';
  import PosterCard from './PosterCard.svelte';

  let { cards, onopen }: { cards: Card[]; onopen: (card: Card) => void } = $props();

  const locale = $derived(getLocale());

  $effect(() =>
    pictures.want(
      cards.map((card) => card.image),
      'w185',
    ),
  );
</script>

<div class="grid">
  {#each cards as card (card.key)}
    <PosterCard {card} {locale} {onopen} />
  {/each}
</div>

<style>
  .grid {
    display: grid;
    grid-template-columns: repeat(6, minmax(0, 1fr));
    gap: 22px 16px;
    padding: 6px;
  }
</style>
