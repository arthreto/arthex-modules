<script lang="ts">
  import { Button, getLocale, hideOverlay, Icon, Skeleton, toast } from '@arthex/sdk';
  import { ageOf, dayOf } from './format';
  import { t } from './i18n';
  import { PERSON_ICON } from './identity';
  import { departmentOf } from './labels';
  import { openAddress, person as loadPerson, problemOf, type Problem } from './library';
  import type { Card, MediaKey, Person } from './media';
  import { pictures } from './pictures.svelte';
  import ProblemState from './ProblemState.svelte';
  import Shelf from './Shelf.svelte';

  interface Props {
    id: number;
    onopen: (key: MediaKey) => void;
    onproblem: (problem: Problem) => void;
  }

  /** Longer biographies are cut to a few lines until asked for. */
  const LONG_BIOGRAPHY = 520;

  let { id, onopen, onproblem }: Props = $props();

  let found = $state<Person | null>(null);
  let problem = $state<Problem | null>(null);
  let wholeBiography = $state(false);

  const locale = $derived(getLocale());
  const photo = $derived(found ? pictures.get(found.image, 'w342') : '');
  const born = $derived.by(() => {
    if (!found?.birthday) return '';
    const gender = found.gender === 1 ? 'female' : 'other';
    const birth = t('born', { gender, date: dayOf(found.birthday, locale), place: found.place || 'none' });
    if (found.deathday) {
      const age = ageOf(found.birthday, new Date(`${found.deathday}T00:00:00`));
      return birth + t('died', { gender, date: dayOf(found.deathday, locale), age });
    }
    return `${birth} ${t('age', { age: ageOf(found.birthday, new Date()) })}`;
  });
  const isLong = $derived((found?.biography.length ?? 0) > LONG_BIOGRAPHY);

  function load() {
    problem = null;
    loadPerson(id).then(
      (person) => {
        found = person;
        pictures.want([person.image], 'w342');
      },
      (error) => {
        problem = problemOf(error);
        onproblem(problem);
      },
    );
  }

  $effect(load);

  async function openOnTmdb() {
    try {
      await openAddress(`https://www.themoviedb.org/person/${id}`);
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
      <span class="photo">
        {#if photo}
          <img src={photo} alt={t('photoOf', { name: found.name })} />
        {:else}
          <span class="none"><Icon path={PERSON_ICON} size={40} stroke={1.4} /></span>
        {/if}
      </span>
      <div class="info">
        {#if found.department}<span class="kind">{departmentOf(found.department)}</span>{/if}
        <h2>{found.name}</h2>
        {#if born}<span class="meta">{born}</span>{/if}
        {#if found.biography}
          <p class:clamped={isLong && !wholeBiography}>{found.biography}</p>
          {#if isLong}
            <button type="button" class="more" onclick={() => (wholeBiography = !wholeBiography)}>
              {wholeBiography ? t('readLess') : t('readMore')}
            </button>
          {/if}
        {/if}
        <div class="buttons"><Button onclick={openOnTmdb}>{t('openTmdb')}</Button></div>
      </div>
    </div>

    <Shelf title={t('sections.knownFor')} cards={found.knownFor} width={116} onopen={openCard} />

    {#if found.credits.length > 0}
      <section>
        <h3>{t('sections.credits')}</h3>
        <div class="credits">
          {#each found.credits as credit (credit.key)}
            <button type="button" class="credit" onclick={() => onopen(credit.key)}>
              <span class="year">{credit.year || '—'}</span>
              <span class="what">
                <span class="title">{credit.title}</span>
                {#if credit.role}<span class="role">{credit.role}</span>{/if}
              </span>
              <span class="role">{t(`kinds.${credit.kind}`)}</span>
            </button>
          {/each}
        </div>
      </section>
    {/if}
  </div>
{:else if problem}
  <ProblemState {problem} onretry={load} />
{:else}
  <div class="page" aria-busy="true">
    <div class="top">
      <Skeleton width="184px" height="276px" radius="12px" />
      <div class="info">
        <Skeleton width="30%" height="14px" />
        <Skeleton width="50%" height="26px" />
        <Skeleton width="70%" height="14px" />
        <Skeleton width="100%" height="96px" />
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

  .photo {
    width: 184px;
    height: 276px;
    flex: none;
    border-radius: 12px;
    overflow: hidden;
    border: 1px solid var(--border-strong);
    background: var(--surface);
  }

  .photo img {
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

  p {
    margin: 2px 0 0;
    font-size: 13.5px;
    line-height: 1.55;
    white-space: pre-line;
  }

  .clamped {
    display: -webkit-box;
    line-clamp: 6;
    -webkit-line-clamp: 6;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .more {
    align-self: flex-start;
    font-size: 13px;
    color: var(--accent-hover);
    border-radius: 4px;
  }

  .more:hover {
    text-decoration: underline;
  }

  .buttons {
    margin-top: 4px;
    display: flex;
    gap: 8px;
  }

  section {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  h3 {
    margin: 0;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-subtle);
  }

  .credits {
    margin: 0 -8px;
    display: flex;
    flex-direction: column;
  }

  .credit {
    width: 100%;
    padding: 8px;
    border-radius: 8px;
    display: flex;
    align-items: center;
    gap: 12px;
    text-align: left;
    transition: background-color var(--motion-fast);
  }

  .credit + .credit {
    border-top: 1px solid var(--border);
  }

  .credit:hover {
    background: var(--surface-hover);
  }

  .credit:focus-visible,
  .more:focus-visible {
    outline: 2px solid var(--accent);
  }

  .year {
    width: 44px;
    flex: none;
    font-size: 12.5px;
    color: var(--text-subtle);
    font-variant-numeric: tabular-nums;
  }

  .what {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .title {
    font-size: 13px;
    font-weight: 500;
  }

  .role {
    font-size: 12px;
    color: var(--text-subtle);
  }
</style>
