<script lang="ts">
  import art from './mascot-art.json';
  import type { Item, Mood, Species } from './companion.svelte';

  interface Props {
    species: Species;
    mood?: Mood;
    wear?: Item | null;
    size?: number;
    label?: string;
  }

  let { species, mood = 'idle', wear = null, size = 200, label }: Props = $props();

  const { common } = art;
  const look = $derived(art.species[species]);
  const eyesOpen = $derived(mood === 'idle' || mood === 'walk' || mood === 'sniff');
  const INK = '#241A17';
</script>

<svg
  class="mascot {mood}"
  role={label ? 'img' : undefined}
  aria-label={label}
  aria-hidden={label ? undefined : true}
  width={size}
  height={size}
  viewBox="0 0 200 200"
  fill="none"
>
  <ellipse cx="100" cy="189" rx="56" ry="7" fill="#000000" opacity=".22" />
  <g class="all">
    <g class="tail">
      <path d={look.tail} fill={look.main} />
      <path d={look.tailTip} fill={look.tipColor} />
    </g>
    <g class="body">
      <path d={common.body} fill={look.main} />
      <path d={common.bib} fill={look.light} />
      <path d={common.pawLeft} fill={look.paw} />
      <path d={common.pawRight} fill={look.paw} />
      {#if wear === 'scarf'}
        <path d={common.scarf} fill="#E5534B" />
        <path d={common.scarfKnot} fill="#C93F38" />
      {:else if wear === 'bow'}
        <path d={common.bowLeft} fill="#7C8CF8" />
        <path d={common.bowRight} fill="#7C8CF8" />
        <circle cx="100" cy="134" r="6" fill="#5B6BE0" />
      {/if}
    </g>
    <g class="head">
      <g class="ear">
        <path
          d={look.ear}
          fill={look.earColor}
          stroke={look.earColor}
          stroke-width="5"
          stroke-linejoin="round"
        />
        <path d={look.earInner} fill={look.inner} />
      </g>
      <g transform="translate(200 0) scale(-1 1)">
        <path
          d={look.ear}
          fill={look.earColor}
          stroke={look.earColor}
          stroke-width="5"
          stroke-linejoin="round"
        />
        <path d={look.earInner} fill={look.inner} />
      </g>
      <path d={look.head} fill={look.main} />
      <path d={look.mask} fill={look.light} />
      {#if look.striped}
        <path d={common.stripes} stroke={look.dark} stroke-width="5" stroke-linecap="round" />
      {/if}
      <ellipse cx="60" cy="108" rx="11" ry="7" fill="#FF8FA3" opacity=".42" />
      <ellipse cx="140" cy="108" rx="11" ry="7" fill="#FF8FA3" opacity=".42" />
      {#if eyesOpen}
        <g class="eyes">
          <ellipse cx="74" cy="90" rx="10.5" ry="12.5" fill={INK} />
          <ellipse cx="126" cy="90" rx="10.5" ry="12.5" fill={INK} />
          <circle cx="78" cy="85" r="4" fill="#FFFFFF" />
          <circle cx="130" cy="85" r="4" fill="#FFFFFF" />
          <circle cx="71" cy="95" r="2" fill="#FFFFFF" opacity=".8" />
          <circle cx="123" cy="95" r="2" fill="#FFFFFF" opacity=".8" />
        </g>
      {:else if mood === 'happy'}
        <path d={common.eyesHappy} stroke={INK} stroke-width="5" stroke-linecap="round" />
      {:else}
        <path d={common.eyesSleep} stroke={INK} stroke-width="4.5" stroke-linecap="round" />
      {/if}
      <path d={common.nose} fill={INK} />
      {#if mood === 'happy'}
        <path d={common.mouthOpen} fill="#7A2E3A" />
        <path d={common.tongue} fill="#FF8FA3" />
      {:else}
        <path d={common.mouth} stroke={INK} stroke-width="2.6" stroke-linecap="round" />
      {/if}
      {#if look.striped}
        <path d={common.whiskers} stroke={look.dark} stroke-width="2.2" stroke-linecap="round" />
      {/if}
      {#if wear === 'glasses'}
        <circle cx="74" cy="90" r="17" stroke="#1F2430" stroke-width="4" fill="#FFFFFF" fill-opacity=".12" />
        <circle cx="126" cy="90" r="17" stroke="#1F2430" stroke-width="4" fill="#FFFFFF" fill-opacity=".12" />
        <path d={common.glassesBridge} stroke="#1F2430" stroke-width="4" />
      {:else if wear === 'crown'}
        <path d={common.crown} fill="#F5C542" stroke="#D9A520" stroke-width="3" stroke-linejoin="round" />
        <circle cx="100" cy="30" r="4" fill="#E5534B" />
      {/if}
    </g>
    {#if mood === 'sleep'}
      <text class="zzz" x="150" y="44" font-size="24" font-weight="700" fill="#C9CDD4">z</text>
    {:else if mood === 'happy'}
      <path class="heart" d={common.heart} fill="#FF6B86" />
    {/if}
  </g>
</svg>

<style>
  .mascot {
    display: block;
    overflow: visible;
  }

  .all {
    transform-origin: 100px 186px;
  }

  .body {
    transform-origin: 100px 186px;
    animation: breathe 3.2s ease-in-out infinite;
  }

  .head {
    transform-origin: 100px 124px;
    animation: bob 3.2s ease-in-out infinite;
  }

  .tail {
    transform-origin: 126px 178px;
    animation: wag 2.2s ease-in-out infinite;
  }

  .ear {
    transform-origin: 76px 56px;
    animation: ear 5.4s ease-in-out infinite;
  }

  .eyes {
    transform-origin: 100px 90px;
    animation: blink 4.8s linear infinite;
  }

  .zzz {
    animation: zzz 2.8s ease-in-out infinite;
  }

  /* Scaled around itself: by default, an SVG shape scales around the corner of the drawing. */
  .heart {
    transform-box: fill-box;
    transform-origin: center;
    animation: heart 1.4s ease-out infinite;
  }

  .happy .all {
    animation: hop 0.62s cubic-bezier(0.3, 0.7, 0.4, 1) infinite;
  }

  .happy .tail {
    animation: wag 0.34s ease-in-out infinite;
  }

  .happy .ear {
    animation: ear 1.2s ease-in-out infinite;
  }

  .sleep .body {
    animation: breathe 5s ease-in-out infinite;
  }

  .sleep .head {
    transform: rotate(6deg) translateY(10px);
    animation: none;
  }

  .sleep .tail,
  .sleep .ear {
    animation: none;
  }

  .sniff .head {
    animation: sniff 1s ease-in-out infinite;
  }

  .sniff .tail {
    animation: wag 1s ease-in-out infinite;
  }

  .walk .all {
    animation: walk 0.5s ease-in-out infinite;
  }

  .walk .tail {
    animation: wag 0.5s ease-in-out infinite;
  }

  @keyframes breathe {
    0%,
    100% {
      transform: scale(1, 1);
    }
    50% {
      transform: scale(1.015, 0.975);
    }
  }

  @keyframes bob {
    0%,
    100% {
      transform: translateY(0);
    }
    50% {
      transform: translateY(2.5px);
    }
  }

  @keyframes wag {
    0%,
    100% {
      transform: rotate(-7deg);
    }
    50% {
      transform: rotate(9deg);
    }
  }

  @keyframes ear {
    0%,
    88%,
    100% {
      transform: rotate(0);
    }
    92% {
      transform: rotate(-9deg);
    }
    96% {
      transform: rotate(3deg);
    }
  }

  @keyframes blink {
    0%,
    93%,
    100% {
      transform: scaleY(1);
    }
    96% {
      transform: scaleY(0.08);
    }
  }

  @keyframes hop {
    0%,
    100% {
      transform: translateY(0) scale(1.03, 0.97);
    }
    45% {
      transform: translateY(-16px) scale(0.98, 1.03);
    }
  }

  @keyframes sniff {
    0%,
    100% {
      transform: translate(0, 5px) rotate(-3deg);
    }
    25% {
      transform: translate(2px, 9px) rotate(3deg);
    }
    50% {
      transform: translate(0, 5px) rotate(-3deg);
    }
    75% {
      transform: translate(-2px, 9px) rotate(-6deg);
    }
  }

  @keyframes walk {
    0%,
    100% {
      transform: translateY(0) rotate(-2.5deg);
    }
    50% {
      transform: translateY(-6px) rotate(2.5deg);
    }
  }

  @keyframes zzz {
    0% {
      opacity: 0;
      transform: translate(0, 6px);
    }
    35% {
      opacity: 1;
    }
    100% {
      opacity: 0;
      transform: translate(10px, -16px);
    }
  }

  @keyframes heart {
    0% {
      opacity: 0;
      transform: translateY(6px) scale(0.7);
    }
    30% {
      opacity: 1;
    }
    100% {
      opacity: 0;
      transform: translateY(-22px) scale(1.1);
    }
  }
</style>
