<script lang="ts">
  import { getLocale } from '@arthex/sdk';
  import type { Card } from './media';
  import { pictures } from './pictures.svelte';
  import PosterCard from './PosterCard.svelte';

  interface Props {
    title: string;
    cards: (Card & { role?: string })[];
    width: number;
    onopen: (card: Card) => void;
  }

  let { title, cards, width, onopen }: Props = $props();

  const locale = $derived(getLocale());

  $effect(() =>
    pictures.want(
      cards.map((card) => card.image),
      'w185',
    ),
  );
</script>

{#if cards.length > 0}
  <section>
    <h3>{title}</h3>
    <div class="shelf scroll">
      {#each cards as card (card.key)}
        <PosterCard {card} detail={card.role} {locale} {width} {onopen} />
      {/each}
    </div>
  </section>
{/if}

<style>
  section {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  h3 {
    margin: 0;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-subtle);
  }

  .shelf {
    display: flex;
    gap: 12px;
    overflow-x: auto;
    overflow-y: hidden;
    padding: 2px 2px 10px;
  }
</style>
