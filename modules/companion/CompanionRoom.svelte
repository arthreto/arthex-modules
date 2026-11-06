<script lang="ts">
  import { Button, getLocale, Segmented, Switch } from '@arthex/sdk';
  import Mascot from './Mascot.svelte';
  import RoomToy from './RoomToy.svelte';
  import { companion, type Frequency, type Mood } from './companion.svelte';
  import { t } from './i18n';
  import { megabytes, millions } from './words';

  const HAPPY_MS = 900;

  const mascot = $derived(companion.mascot);
  const flair = $derived(companion.flair);
  let happy = $state(false);
  let asking = $state(false);
  let pops = $state<number[]>([]);
  let happyTimer: ReturnType<typeof setTimeout> | undefined;

  const mood = $derived<Mood>(happy ? 'happy' : mascot?.onScreen ? 'idle' : 'sleep');
  const number = (value: number) => new Intl.NumberFormat(getLocale()).format(value);
  const size = $derived(flair ? t('flair.size', { size: megabytes(flair.bytes) }) : '');
  const frequencies = $derived<{ value: Frequency; label: string }[]>([
    { value: 'rare', label: t('frequency.rare') },
    { value: 'normal', label: t('frequency.normal') },
    { value: 'often', label: t('frequency.often') },
  ]);

  function tap(): void {
    happy = true;
    pops = [...pops.slice(-3), Date.now()];
    clearTimeout(happyTimer);
    happyTimer = setTimeout(() => {
      happy = false;
      pops = [];
    }, HAPPY_MS);
  }

  function toggleFlair(on: boolean): void {
    if (flair?.state === 'off' && on) void companion.downloadFlair();
    else if (flair?.state === 'loading') void companion.deleteFlair();
    else if (flair?.state === 'ready' && !on) asking = true;
  }

  function deleteFlair(): void {
    asking = false;
    void companion.deleteFlair();
  }
</script>

{#if mascot?.species}
  <div class="room">
    <div class="floor"></div>
    <svg class="decor" aria-hidden="true" viewBox="0 0 1080 538" preserveAspectRatio="xMidYMax slice">
      <rect
        x="735"
        y="76"
        width="210"
        height="196"
        rx="16"
        fill="#14233A"
        stroke="rgba(255,255,255,.2)"
        stroke-width="6"
      />
      <circle cx="878" cy="130" r="27" fill="#F5E6B8" />
      <circle cx="866" cy="124" r="24" fill="#14233A" />
      <circle cx="772" cy="118" r="2.5" fill="#FFFFFF" opacity=".8" />
      <circle cx="806" cy="160" r="2" fill="#FFFFFF" opacity=".6" />
      <circle cx="902" cy="208" r="2.5" fill="#FFFFFF" opacity=".7" />
      <circle cx="778" cy="222" r="1.8" fill="#FFFFFF" opacity=".6" />
      <path d="M840 76 V272 M735 174 H945" stroke="rgba(255,255,255,.2)" stroke-width="5" />
      <rect x="721" y="272" width="238" height="11" rx="5" fill="rgba(255,255,255,.2)" />
      <path d="M60 252 H226" stroke="rgba(255,255,255,.22)" stroke-width="8" stroke-linecap="round" />
      <rect x="82" y="204" width="18" height="44" rx="2" fill="#7C8CF8" opacity=".8" />
      <rect x="102" y="194" width="16" height="54" rx="2" fill="#FDBA74" opacity=".8" />
      <rect x="120" y="208" width="20" height="40" rx="2" fill="#6EE7B7" opacity=".7" />
      <path d="M186 248 C184 234 190 226 195 222 C197 232 202 240 195 248 Z" fill="#6EE7B7" opacity=".8" />
    </svg>
    <div class="rug"></div>

    <div class="top">
      <div class="identity">
        <span class="title">
          <span class="name">{mascot.name}</span>
          <span class="level">{t('level', { level: mascot.level })}</span>
        </span>
        <span
          class="track"
          role="progressbar"
          aria-label={t('xpLabel')}
          aria-valuenow={mascot.xp}
          aria-valuemin={0}
          aria-valuemax={mascot.xpNeeded}
        >
          {#key mascot.level}
            <span class="done" style:width="{(mascot.xp / mascot.xpNeeded) * 100}%"></span>
          {/key}
        </span>
        <span class="xp">{mascot.xp} / {mascot.xpNeeded} XP</span>
      </div>
      <div class="purse">
        <span class="kibble"><span class="coin" aria-hidden="true"></span>{number(mascot.kibble)}</span>
        <Button onclick={() => (companion.view = 'shop')}>{t('shop.title')}</Button>
      </div>
    </div>

    <button class="pet" aria-label={t('pet.action', { name: mascot.name })} onclick={tap}>
      <Mascot species={mascot.species} {mood} wear={mascot.wear} size={270} />
    </button>
    {#each pops as pop, index (pop)}
      <span class="pop" style:left="{40 + index * 7}%">♥</span>
    {/each}

    <RoomToy game="hide" left={112} />
    <RoomToy game="catch" left={246} />
    <RoomToy game="memory" right={318} />
    <RoomToy game="pet" right={204} />
    <RoomToy game="flair" right={90} dimmed={flair?.state !== 'ready'} />

    <div class="panel screen">
      <span>{t('screen.label')}</span>
      <Switch
        checked={mascot.onScreen}
        label={t('screen.label')}
        onchange={(on) => void companion.saveScreen(on, mascot.frequency)}
      />
      {#if mascot.onScreen}
        <Segmented
          options={frequencies}
          value={mascot.frequency}
          label={t('frequency.label')}
          onchange={(frequency) => void companion.saveScreen(true, frequency)}
        />
      {/if}
    </div>

    {#if flair && companion.owns('flair')}
      <div class="panel flair">
        <span>{t('games.flair')}</span>
        {#if asking}
          <span class="note">{t('flair.askDelete', { size: megabytes(flair.bytes) })}</span>
          <Button size="small" onclick={() => (asking = false)}>{t('flair.keep')}</Button>
          <Button size="small" variant="quiet-danger" onclick={deleteFlair}>{t('flair.delete')}</Button>
        {:else if flair.state === 'loading'}
          <span
            class="track grow"
            role="progressbar"
            aria-label={t('flair.downloading')}
            aria-valuenow={flair.percent}
            aria-valuemin={0}
            aria-valuemax={100}
          >
            <span class="done" style:width="{flair.percent}%"></span>
          </span>
          <span class="percent">{flair.percent} %</span>
          <Button size="small" onclick={() => void companion.deleteFlair()}>{t('flair.cancel')}</Button>
        {:else if flair.state === 'ready'}
          <span class="note">{t('flair.words', { count: number(millions(flair.words)) })}</span>
          <Button size="small" variant="primary" onclick={() => companion.play('flair')}>{t('play')}</Button>
        {:else}
          <span class="note">{size}</span>
        {/if}
        {#if !asking}
          <Switch checked={flair.state !== 'off'} label={t('games.flair')} onchange={toggleFlair} />
        {/if}
      </div>
    {/if}
  </div>
{/if}

<style>
  .room {
    position: absolute;
    inset: 0;
    overflow: hidden;
    border-radius: 16px;
    background: var(--surface);
    border: 1px solid var(--border);
  }

  .floor {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    height: 28%;
    background: var(--surface-sunken);
    border-top: 1px solid var(--border);
  }

  .decor {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
  }

  .rug {
    position: absolute;
    left: 50%;
    bottom: 7%;
    width: 320px;
    height: 60px;
    margin-left: -160px;
    border-radius: 50%;
    background: var(--surface-sunken);
  }

  .top {
    position: absolute;
    left: 20px;
    top: 16px;
    right: 20px;
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
  }

  .identity {
    width: 250px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .title {
    display: flex;
    align-items: baseline;
    gap: 10px;
  }

  .name {
    font-size: 24px;
    font-weight: 700;
    letter-spacing: -0.01em;
  }

  .level {
    font-size: 13px;
    color: var(--text-muted);
  }

  .track {
    display: block;
    height: 7px;
    border-radius: 4px;
    background: var(--surface-hover);
    overflow: hidden;
  }

  .grow {
    flex: 1;
    min-width: 0;
    height: 6px;
  }

  .done {
    display: block;
    height: 100%;
    border-radius: 4px;
    background: var(--accent);
    transition: width 300ms ease-out;
  }

  .xp {
    font-size: 12px;
    color: var(--text-subtle);
  }

  .purse {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .kibble {
    height: 36px;
    padding: 0 12px;
    border-radius: 18px;
    background: var(--surface-sunken);
    display: inline-flex;
    align-items: center;
    gap: 7px;
    font-size: 13.5px;
    font-weight: 600;
  }

  .coin {
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: #c98a4b;
    border: 2px solid #8a5a2b;
  }

  .pet {
    position: absolute;
    left: 50%;
    bottom: 7%;
    margin-left: -135px;
    width: 270px;
    height: 270px;
  }

  .pop {
    position: absolute;
    bottom: 46%;
    font-size: 15px;
    font-weight: 700;
    color: var(--tint-green);
    pointer-events: none;
    animation: pop 700ms ease-out forwards;
  }

  @keyframes pop {
    0% {
      opacity: 0;
      transform: translateY(8px) scale(0.8);
    }
    25% {
      opacity: 1;
    }
    100% {
      opacity: 0;
      transform: translateY(-46px);
    }
  }

  .panel {
    position: absolute;
    bottom: 14px;
    min-height: 46px;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 12px;
    border-radius: 12px;
    background: var(--surface-sunken);
    font-size: 13px;
  }

  .screen {
    left: 16px;
  }

  .flair {
    right: 16px;
    width: 356px;
  }

  .note {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    font-size: 12px;
    color: var(--text-subtle);
  }

  .percent {
    flex: none;
    font-size: 12px;
    color: var(--text-subtle);
    font-variant-numeric: tabular-nums;
  }

  .screen :global(.segmented) {
    align-self: center;
    padding: 2px;
  }

  .screen :global(.segmented button) {
    height: 26px;
    padding: 0 10px;
  }
</style>
