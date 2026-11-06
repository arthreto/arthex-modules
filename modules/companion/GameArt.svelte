<script lang="ts">
  import type { ShopGame } from './companion.svelte';

  interface Props {
    game: ShopGame;
    size: number;
  }

  let { game, size }: Props = $props();
</script>

<svg aria-hidden="true" width={size} height={size} viewBox="0 0 100 100">
  {#if game === 'hide'}
    <ellipse cx="50" cy="92" rx="42" ry="6" fill="#000000" opacity=".22" />
    <path d="M18 46 H82 V90 H18 Z" fill="#C8965C" />
    <path d="M18 46 H82 V54 H18 Z" fill="#33241B" />
    <circle cx="41" cy="50" r="3.2" fill="#FFFFFF" />
    <circle cx="59" cy="50" r="3.2" fill="#FFFFFF" />
    <path d="M18 46 L4 32 L32 26 L46 46 Z" fill="#DDB07A" />
    <path d="M82 46 L96 32 L68 26 L54 46 Z" fill="#DDB07A" />
    <path d="M44 56 H56 V90 H44 Z" fill="#E7C89B" opacity=".75" />
  {:else if game === 'catch'}
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
  {:else if game === 'memory'}
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
  {:else if game === 'pet'}
    <ellipse cx="50" cy="92" rx="38" ry="5" fill="#000000" opacity=".2" />
    <path
      d="M50 92 C18 74 6 56 12 38 C18 22 42 22 50 40 C58 22 82 22 88 38 C94 56 82 74 50 92 Z"
      fill="#F48FB1"
    />
    <path
      d="M50 80 C28 67 19 54 23 42 C27 32 42 33 50 46 C58 33 73 32 77 42 C81 54 72 67 50 80 Z"
      fill="#F9A8D4"
    />
  {:else}
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
  {/if}
</svg>

<style>
  svg {
    display: block;
  }
</style>
