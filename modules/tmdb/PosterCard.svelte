<script lang="ts">
  import { Icon } from '@arthex/sdk';
  import { ratingOf } from './format';
  import { ICON, PERSON_ICON, TV_ICON } from './identity';
  import { departmentOf } from './labels';
  import type { Card } from './media';
  import { pictures } from './pictures.svelte';

  interface Props {
    card: Card;
    /** Under the name, in place of the year and rating: a role in a cast. */
    detail?: string;
    locale: string;
    /** In a row that scrolls, rather than in a grid. */
    width?: number;
    onopen: (card: Card) => void;
  }

  let { card, detail, locale, width, onopen }: Props = $props();

  const image = $derived(pictures.get(card.image, 'w185'));
  const isPerson = $derived(card.kind === 'person');
  const sub = $derived(
    detail ??
      (isPerson
        ? departmentOf(card.department)
        : [card.year, card.rating ? `★ ${ratingOf(card.rating, locale)}` : ''].filter(Boolean).join(' · ')),
  );
</script>

<button
  type="button"
  class="card"
  class:fixed={width !== undefined}
  style:width={width === undefined ? undefined : `${width}px`}
  aria-label={[card.title, sub].filter(Boolean).join(', ')}
  onclick={() => onopen(card)}
>
  <span class="frame" class:person={isPerson}>
    {#if image}
      <img src={image} alt="" />
    {:else}
      <span class="none"
        ><Icon
          path={isPerson ? PERSON_ICON : card.kind === 'tv' ? TV_ICON : ICON}
          size={26}
          stroke={1.5}
        /></span
      >
    {/if}
  </span>
  <span class="name">{card.title}</span>
  {#if sub}<span class="sub">{sub}</span>{/if}
</button>

<style>
  .card {
    min-width: 0;
    margin: -6px;
    padding: 6px;
    border-radius: 10px;
    display: flex;
    flex-direction: column;
    gap: 7px;
    text-align: left;
    transition: background-color var(--motion-fast);
  }

  .card:hover {
    background: var(--surface-hover);
  }

  .card:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 0;
  }

  .fixed {
    flex: none;
    margin: 0;
  }

  .frame {
    aspect-ratio: 2 / 3;
    border-radius: 8px;
    overflow: hidden;
    border: 1px solid var(--border-strong);
    background: var(--surface);
  }

  .person {
    border-radius: 10px;
  }

  img {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .none {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-subtle);
  }

  .name {
    font-size: 13px;
    font-weight: 500;
    line-height: 1.3;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .fixed .name {
    white-space: normal;
  }

  .sub {
    margin-top: -4px;
    font-size: 12px;
    line-height: 1.3;
    color: var(--text-subtle);
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
