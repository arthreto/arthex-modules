<script lang="ts">
  import { IconButton, TextField, type ToolProps } from '@arthex/sdk';
  import { onMount } from 'svelte';
  import HomePage from './HomePage.svelte';
  import { t } from './i18n';
  import { KEY_ICON } from './identity';
  import { parseInput } from './input';
  import { hasKey, type Problem } from './library';
  import { parseKey, type Card, type MediaKey } from './media';
  import PersonPage from './PersonPage.svelte';
  import KeyPage from './KeyPage.svelte';
  import ResultsPage from './ResultsPage.svelte';
  import TitlePage from './TitlePage.svelte';

  type Entry = { view: 'home' } | { view: 'results'; query: string } | { view: 'page'; key: MediaKey };

  /** Typing waits this long before asking TMDB, so that a word is not searched letter by letter. */
  const TYPING_PAUSE_MS = 350;

  let { input }: ToolProps = $props();

  // What a search result passed on is only where the tool starts.
  // svelte-ignore state_referenced_locally
  const start = parseInput(input);

  let stack = $state<Entry[]>([
    { view: 'home' },
    ...(start && 'page' in start ? [{ view: 'page', key: start.page } as const] : []),
    ...(start && 'search' in start ? [{ view: 'results', query: start.search } as const] : []),
  ]);
  let query = $state(start && 'search' in start ? start.search : '');
  let keySaved = $state<boolean | null>(null);
  let keyOpen = $state(false);
  let refused = $state(false);
  let typing: ReturnType<typeof setTimeout> | undefined;

  const here = $derived<Entry>(stack[stack.length - 1] ?? { view: 'home' });
  const showKey = $derived(keySaved === false || keyOpen);
  const page = $derived(here.view === 'page' ? parseKey(here.key) : null);

  onMount(() => {
    hasKey().then(
      (saved) => (keySaved = saved),
      () => (keySaved = false),
    );
    return () => clearTimeout(typing);
  });

  function go(entry: Entry) {
    stack = [...stack, entry];
  }

  const open = (key: MediaKey) => go({ view: 'page', key });
  const openCard = (card: Card) => open(card.key);

  function back() {
    stack = stack.slice(0, -1);
    const top = stack[stack.length - 1];
    if (top?.view === 'results') query = top.query;
  }

  function typed() {
    clearTimeout(typing);
    typing = setTimeout(() => {
      const asked = query.trim();
      const kept = stack.filter((entry) => entry.view !== 'results');
      stack = asked ? [...kept, { view: 'results', query: asked }] : kept;
    }, TYPING_PAUSE_MS);
  }

  function problem(found: Problem) {
    if (found !== 'no-key' && found !== 'refused') return;
    keySaved = found === 'refused';
    refused = found === 'refused';
    keyOpen = true;
  }

  function keyChanged(saved: boolean) {
    keySaved = saved;
    keyOpen = false;
    refused = false;
    if (!saved) {
      stack = [{ view: 'home' }];
      query = '';
    }
  }
</script>

<div class="tool">
  <div class="bar">
    <IconButton
      icon="chevronLeft"
      label={t('back')}
      size={38}
      disabled={stack.length < 2 || showKey}
      onclick={back}
    />
    <div class="search">
      <TextField
        label={t('searchLabel')}
        hideLabel
        bind:value={query}
        oninput={typed}
        placeholder={t('searchPlaceholder')}
        spellcheck="false"
        autocomplete="off"
        disabled={!keySaved}
      />
    </div>
    <IconButton
      path={KEY_ICON}
      label={t('key.button')}
      size={38}
      active={showKey}
      onclick={() => (keyOpen = !keyOpen)}
    />
  </div>

  {#if keySaved === null}
    <div class="wait"></div>
  {:else if showKey}
    <KeyPage
      saved={keySaved}
      {refused}
      onsaved={() => keyChanged(true)}
      onremoved={() => keyChanged(false)}
    />
  {:else}
    {#key stack.length}
      {#if here.view === 'results'}
        <ResultsPage query={here.query} onopen={openCard} onproblem={problem} />
      {:else if page && page.kind === 'person'}
        <PersonPage id={page.id} onopen={open} onproblem={problem} />
      {:else if page && page.kind !== 'person'}
        <TitlePage kind={page.kind} id={page.id} onopen={open} onproblem={problem} />
      {:else}
        <HomePage onopen={openCard} onproblem={problem} />
      {/if}
    {/key}
  {/if}
</div>

<style>
  .tool {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .bar {
    flex: none;
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .search {
    flex: 1;
    min-width: 0;
  }

  .search :global(input) {
    width: 100%;
    height: 40px;
  }

  .wait {
    flex: 1;
  }
</style>
