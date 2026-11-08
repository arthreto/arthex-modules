<script lang="ts">
  import { Button, getLocale, hideOverlay, Icon, Skeleton, toast } from '@arthex/sdk';
  import Episodes from './Episodes.svelte';
  import { dayOf, durationOf, ratingOf } from './format';
  import { t } from './i18n';
  import { ICON, PLAY, STAR, TV_ICON } from './identity';
  import { certificationOf, statusOf } from './labels';
  import { openAddress, problemOf, title as loadTitle, type Problem } from './library';
  import type { Card, MediaKey, Title } from './media';
  import { pictures } from './pictures.svelte';
  import ProblemState from './ProblemState.svelte';
  import Shelf from './Shelf.svelte';

  interface Props {
    kind: 'movie' | 'tv';
    id: number;
    onopen: (key: MediaKey) => void;
    onproblem: (problem: Problem) => void;
  }

  let { kind, id, onopen, onproblem }: Props = $props();

  let found = $state<Title | null>(null);
  let problem = $state<Problem | null>(null);

  const locale = $derived(getLocale());
  const poster = $derived(found ? pictures.get(found.image, 'w342') : '');
  const meta = $derived.by(() => {
    if (!found) return '';
    const parts =
      found.kind === 'movie'
        ? [
            dayOf(found.date, locale),
            found.runtime ? durationOf(found.runtime) : '',
            certificationOf(found.certification),
          ]
        : [
            found.year,
            found.seasonCount ? t('seasons', { count: found.seasonCount, episodes: found.episodeCount }) : '',
            statusOf(found.status),
          ];
    return parts.filter(Boolean).join(' · ');
  });
  const providerGroups = $derived(
    found
      ? (['stream', 'rent', 'buy'] as const)
          .map((group) => ({ group, list: found?.providers[group] ?? [] }))
          .filter(({ list }) => list.length > 0)
      : [],
  );

  function load() {
    problem = null;
    loadTitle(kind, id).then(
      (title) => {
        found = title;
        pictures.want([title.image], 'w342');
      },
      (error) => {
        problem = problemOf(error);
        onproblem(problem);
      },
    );
  }

  $effect(load);

  $effect(() => {
    if (found)
      pictures.want(
        Object.values(found.providers)
          .flat()
          .map((provider) => provider.logo),
        'w92',
      );
  });

  async function open(address: string) {
    try {
      await openAddress(address);
      hideOverlay();
    } catch {
      toast(t('failed'));
    }
  }

  const openCard = (card: Card) => onopen(card.key);
</script>

{#if found}
  <div class="page scroll fade-in">
    <div class="top">
      <span class="poster">
        {#if poster}
          <img src={poster} alt={t('posterOf', { title: found.title })} />
        {:else}
          <span class="none"><Icon path={found.kind === 'tv' ? TV_ICON : ICON} size={40} stroke={1.4} /></span
          >
        {/if}
      </span>
      <div class="info">
        <span class="kind">{t(`kinds.${found.kind}`)}</span>
        <h2>{found.title}</h2>
        {#if meta}<span class="meta">{meta}</span>{/if}
        {#if found.genres.length > 0}
          <div class="genres">
            {#each found.genres as genre (genre)}<span class="genre">{genre}</span>{/each}
          </div>
        {/if}
        <span class="rating">
          <span class="star"><Icon path={STAR} size={15} filled /></span>
          <strong>{found.rating ? t('outOfTen', { rating: ratingOf(found.rating, locale) }) : '—'}</strong>
          {#if found.votes}<span class="subtle">{t('votes', { count: found.votes })}</span>{/if}
        </span>
        {#if found.makers.length > 0}
          <span class="makers">
            {t(`makers.${found.kind}`)}
            {#each found.makers as maker, index (maker.key)}
              <button type="button" class="link" onclick={() => onopen(maker.key)}>{maker.name}</button
              >{index < found.makers.length - 1 ? ', ' : ''}
            {/each}
          </span>
        {/if}
        <p>{found.overview || t('noOverview')}</p>
        <div class="buttons">
          {#if found.trailers[0]}
            {@const trailer = found.trailers[0]}
            <Button variant="primary" onclick={() => open(trailer.address)}>
              <Icon path={PLAY} size={14} filled />{t('trailer')}
            </Button>
          {/if}
          <Button
            onclick={() => open(`https://www.themoviedb.org/${found?.kind}/${found?.key.split(':')[1]}`)}
          >
            {t('openTmdb')}
          </Button>
        </div>
      </div>
    </div>

    <Shelf title={t('sections.cast')} cards={found.cast} width={104} onopen={openCard} />

    {#if found.kind === 'tv' && found.seasons.length > 0}
      <Episodes id={Number(found.key.split(':')[1])} seasons={found.seasons} />
    {/if}

    {#if providerGroups.length > 0}
      <section>
        <h3>{t('sections.providers')}</h3>
        {#each providerGroups as { group, list } (group)}
          <div class="providers">
            <span class="subtle">{t(`providers.${group}`)}</span>
            <div class="logos">
              {#each list as provider (provider.name)}
                {@const logo = pictures.get(provider.logo, 'w92')}
                {#if logo}<img
                    src={logo}
                    alt={provider.name}
                    title={provider.name}
                    width="34"
                    height="34"
                  />{/if}
              {/each}
            </div>
          </div>
        {/each}
      </section>
    {/if}

    <Shelf
      title={t('sections.recommendations')}
      cards={found.recommendations}
      width={116}
      onopen={openCard}
    />
  </div>
{:else if problem}
  <ProblemState {problem} onretry={load} />
{:else}
  <div class="page" aria-busy="true">
    <div class="top">
      <Skeleton width="184px" height="276px" radius="12px" />
      <div class="info">
        <Skeleton width="30%" height="14px" />
        <Skeleton width="60%" height="26px" />
        <Skeleton width="45%" height="14px" />
        <Skeleton width="100%" height="64px" />
      </div>
    </div>
  </div>
{/if}

<style>
  .page {
    flex: 1;
    min-height: 0;
    overflow-x: hidden;
    overflow-y: auto;
    padding-right: 6px;
    display: flex;
    flex-direction: column;
    gap: 22px;
  }

  .top {
    display: flex;
    gap: 24px;
  }

  .poster {
    width: 184px;
    height: 276px;
    flex: none;
    border-radius: 12px;
    overflow: hidden;
    border: 1px solid var(--border-strong);
    background: var(--surface);
  }

  .poster img {
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

  .info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .kind {
    font-size: 11.5px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-subtle);
  }

  h2 {
    margin: -4px 0 0;
    font-size: 24px;
    font-weight: 600;
    line-height: 1.2;
    letter-spacing: -0.01em;
  }

  .meta {
    font-size: 13px;
    color: var(--text-muted);
  }

  .genres {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .genre {
    height: 28px;
    padding: 0 12px;
    border-radius: var(--radius-pill);
    background: var(--surface);
    border: 1px solid var(--border-strong);
    font-size: 12.5px;
    color: var(--text-muted);
    display: inline-flex;
    align-items: center;
  }

  .rating {
    display: flex;
    align-items: center;
    gap: 7px;
    font-size: 13px;
  }

  .star {
    display: flex;
    color: var(--tint-amber);
  }

  .subtle,
  .makers {
    font-size: 13px;
    color: var(--text-subtle);
  }

  .link {
    color: var(--text);
    border-radius: 4px;
  }

  .link:hover {
    color: var(--accent-hover);
    text-decoration: underline;
  }

  .link:focus-visible {
    outline: 2px solid var(--accent);
  }

  p {
    margin: 2px 0 0;
    font-size: 13.5px;
    line-height: 1.55;
  }

  .buttons {
    margin-top: 4px;
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

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

  .providers {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .providers .subtle {
    width: 96px;
    flex: none;
    font-size: 12.5px;
  }

  .logos {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .logos img {
    display: block;
    border-radius: 8px;
  }
</style>
