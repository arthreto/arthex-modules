<script lang="ts">
  import { TextField } from '@arthex/sdk';
  import type { HTMLInputAttributes } from 'svelte/elements';

  interface Props extends Omit<HTMLInputAttributes, 'value'> {
    label: string;
    value: string;
    error?: string | null;
    mono?: boolean;
    onvalue?: (value: string) => void;
  }

  let { label, value, error = null, mono = false, onvalue, ...rest }: Props = $props();
</script>

<div class="compact" class:invalid={error} class:mono>
  <TextField
    {label}
    {value}
    spellcheck="false"
    autocomplete="off"
    aria-invalid={error ? true : undefined}
    oninput={(event) => onvalue?.(event.currentTarget.value)}
    {...rest}
  />
  {#if error}<span class="error">{error}</span>{/if}
</div>

<style>
  .compact {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 5px;
  }

  .compact :global(label) {
    gap: 5px;
    font-size: 12.5px;
  }

  .compact :global(input) {
    width: 100%;
    min-width: 0;
    height: 36px;
    box-sizing: border-box;
    padding: 0 12px;
    font-size: 13.5px;
  }

  .compact :global(input:disabled) {
    opacity: 0.55;
  }

  .mono :global(input) {
    font-family: var(--font-mono);
    font-size: 13px;
  }

  .invalid :global(input) {
    border-color: var(--tint-red);
  }

  .error {
    font-size: 12px;
    color: var(--tint-red);
  }
</style>
