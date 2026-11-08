<script lang="ts">
  import { getLocale, Segmented, Skeleton } from '@arthex/sdk';
  import { dayOf, durationOf } from './format';
  import { t } from './i18n';
  import { season } from './library';
  import type { Episode } from './media';
  import { pictures } from './pictures.svelte';

  let { id, seasons }: { id: number; seasons: number[] } = $props();

  // svelte-ignore state_referenced_locally
  let chosen = $state(seasons[0] ?? 1);
  let episodes = $state<Episode[] | null>(null);
  let failed = $state(false);

  const locale = $derived(getLocale());
  const options = $derived(
    seasons.map((number) => ({ value: String(number), label: t('season', { number }) })),
  );

  $effect(() => {
    const asked = chosen;
    episodes = null;
    failed = false;
    season(id, asked).then(
      (found) => {
        if (asked !== chosen) return;
        episodes = found;
        pictures.want(
          found.map((episode) => episode.image),
          'w300',
        );
      },
      () => (failed = true),
    );
  });
</script>

<section>
  <div class="heading">
    <h3>{t('sections.episodes')}</h3>
    {#if seasons.length > 1}
      <Segmented
        {options}
        value={String(chosen)}
        label={t('seasonLabel')}
        onchange={(value) => (chosen = Number(value))}
      />
    {/if}
  </div>
  <div class="list">
    {#if episodes}
      {#each episodes as episode (episode.number)}
        {@const still = pictures.get(episode.image, 'w300')}
        <div class="episode">
          {#if episode.image}
            <span class="still"
              >{#if still}<img src={still} alt="" />{/if}</span
            >
          {/if}
          <span class="text">
            <span class="title">{episode.number}. {episode.title}</span>
            <span class="meta">
              {[dayOf(episode.date, locale), episode.runtime ? durationOf(episode.runtime) : '']
                .filter(Boolean)
                .join(' · ')}
            </span>
            {#if episode.overview}<span class="overview">{episode.overview}</span>{/if}
          </span>
        </div>
      {/each}
    {:else if failed}
      <span class="meta">{t('failed')}</span>
    {:else}
      {#each [0, 1, 2] as row (row)}
        <div class="episode">
          <Skeleton width="128px" height="72px" radius="8px" /><Skeleton width="60%" height="16px" />
        </div>
      {/each}
    {/if}
  </div>
</section>

<style>
  section {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .heading {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  h3 {
    margin: 0;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-subtle);
  }

  .list {
    padding: 6px;
    border-radius: 14px;
    background: var(--surface);
    border: 1px solid var(--border);
    display: flex;
    flex-direction: column;
  }

  .episode {
    padding: 8px;
    display: flex;
    gap: 14px;
  }

  .episode + .episode {
    border-top: 1px solid var(--border);
  }

  .still {
    width: 128px;
    height: 72px;
    flex: none;
    border-radius: 8px;
    overflow: hidden;
    border: 1px solid var(--border-strong);
    background: var(--surface);
  }

  img {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .title {
    font-size: 13px;
    font-weight: 600;
  }

  .meta {
    font-size: 12px;
    color: var(--text-subtle);
  }

  .overview {
    font-size: 12.5px;
    line-height: 1.45;
    color: var(--text-muted);
    display: -webkit-box;
    line-clamp: 2;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
</style>
