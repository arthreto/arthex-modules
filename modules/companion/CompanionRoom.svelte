<script lang="ts">
  import { Button, getLocale, Segmented, Switch } from '@arthex/sdk';
  import Mascot from './Mascot.svelte';
  import RoomToy from './RoomToy.svelte';
  import { companion, type Frequency, type GameId, type Mood } from './companion.svelte';
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
    companion.tap();
    happy = true;
    pops = [...pops.slice(-3), Date.now()];
    clearTimeout(happyTimer);
    happyTimer = setTimeout(() => {
      happy = false;
      pops = [];
    }, HAPPY_MS);
  }

  function play(game: GameId): void {
    companion.view = game;
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
          <span class="done" style:width="{(mascot.xp / mascot.xpNeeded) * 100}%"></span>
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
      <span class="pop" style:left="{40 + index * 7}%">+1 XP</span>
    {/each}

    <RoomToy label={t('games.hide')} left={112} onclick={() => void companion.hideAndSeek()}>
      <ellipse cx="50" cy="92" rx="42" ry="6" fill="#000000" opacity=".22" />
      <path d="M18 46 H82 V90 H18 Z" fill="#C8965C" />
      <path d="M18 46 H82 V54 H18 Z" fill="#33241B" />
      <circle cx="41" cy="50" r="3.2" fill="#FFFFFF" />
      <circle cx="59" cy="50" r="3.2" fill="#FFFFFF" />
      <path d="M18 46 L4 32 L32 26 L46 46 Z" fill="#DDB07A" />
      <path d="M82 46 L96 32 L68 26 L54 46 Z" fill="#DDB07A" />
      <path d="M44 56 H56 V90 H44 Z" fill="#E7C89B" opacity=".75" />
    </RoomToy>
    <RoomToy label={t('games.catch')} left={246} onclick={() => play('catch')}>
      <ellipse cx="50" cy="92" rx="40" ry="6" fill="#000000" opacity=".22" />
      <g fill="#C98A4B" stroke="#8A5A2B" stroke-width="1.5">
        <circle cx="34" cy="50" r="7" />
        <circle cx="50" cy="44" r="7" />
        <circle cx="66" cy="50" r="7" />
        <circle cx="42" cy="54" r="7" />
        <circle cx="58" cy="54" r="7" />
      </g>
      <ellipse cx="50" cy="60" rx="38" ry="9" fill="#3F4DB8" />
      <path d="M12 60 C20 70 80 70 88 60 L78 88 C76 94 24 94 22 88 Z" fill="#6474EE" />
      <path
        d="M28 76 C40 80 60 80 72 76"
        stroke="#FFFFFF"
        stroke-width="3"
        stroke-linecap="round"
        opacity=".5"
        fill="none"
      />
    </RoomToy>
    <RoomToy label={t('games.memory')} right={318} onclick={() => play('memory')}>
      <ellipse cx="50" cy="92" rx="38" ry="5" fill="#000000" opacity=".2" />
      <g transform="rotate(-20 50 88)">
        <rect x="30" y="26" width="40" height="60" rx="7" fill="#7C8CF8" />
        <circle cx="50" cy="56" r="9" fill="#FFFFFF" opacity=".35" />
      </g>
      <g transform="rotate(18 50 88)">
        <rect x="30" y="26" width="40" height="60" rx="7" fill="#C4B5FD" />
        <circle cx="50" cy="56" r="9" fill="#FFFFFF" opacity=".4" />
      </g>
      <rect x="30" y="24" width="40" height="62" rx="7" fill="#F3F4F6" />
      <path
        d="M50 66 C40 59 36 54 38 49 C40 44 47 44 50 50 C53 44 60 44 62 49 C64 54 60 59 50 66 Z"
        fill="#E5534B"
      />
    </RoomToy>
    <RoomToy label={t('games.pet')} right={204} onclick={() => play('pet')}>
      <ellipse cx="50" cy="92" rx="38" ry="5" fill="#000000" opacity=".2" />
      <path
        d="M50 92 C18 74 6 56 12 38 C18 22 42 22 50 40 C58 22 82 22 88 38 C94 56 82 74 50 92 Z"
        fill="#F48FB1"
      />
      <path
        d="M50 80 C28 67 19 54 23 42 C27 32 42 33 50 46 C58 33 73 32 77 42 C81 54 72 67 50 80 Z"
        fill="#F9A8D4"
      />
    </RoomToy>
    <RoomToy
      label={t('games.flair')}
      right={90}
      dimmed={flair?.state !== 'ready'}
      onclick={() => {
        if (flair?.state === 'ready') play('flair');
      }}
    >
      <ellipse cx="50" cy="92" rx="42" ry="5" fill="#000000" opacity=".2" />
      <path d="M6 42 V86 C22 82 38 82 50 90 C62 82 78 82 94 86 V42 Z" fill="#C4632A" />
      <path
        d="M50 42 C38 32 24 30 11 34 V80 C24 76 38 77 50 85 C62 77 76 76 89 80 V34 C76 30 62 32 50 42 Z"
        fill="#F7F3EA"
      />
      <path d="M50 42 V85" stroke="#D8D0BE" stroke-width="2" />
      <path
        d="M20 44 C28 42 36 44 42 48 M20 54 C28 52 36 54 42 58 M20 64 C28 62 36 64 42 68 M80 44 C72 42 64 44 58 48 M80 54 C72 52 64 54 58 58 M80 64 C72 62 64 64 58 68"
        stroke="#A9A18F"
        stroke-width="2.5"
        stroke-linecap="round"
        fill="none"
      />
    </RoomToy>

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

    {#if flair}
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
          <Button size="small" variant="primary" onclick={() => play('flair')}>{t('play')}</Button>
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
