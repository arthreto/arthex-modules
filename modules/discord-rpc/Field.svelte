<script lang="ts">
  import { TextField } from '@arthex/sdk';
  import { t } from './i18n';
  import type { FieldError } from './template';

  interface Props {
    label: string;
    value: string;
    hideLabel?: boolean;
    placeholder?: string;
    type?: 'text' | 'number' | 'datetime-local';
    maxlength?: number;
    error?: FieldError | null;
    onvalue: (value: string) => void;
  }

  let {
    label,
    value,
    hideLabel = false,
    placeholder,
    type = 'text',
    maxlength,
    error = null,
    onvalue,
  }: Props = $props();
</script>

<div class="compact" class:invalid={error}>
  <TextField
    {label}
    {hideLabel}
    {value}
    {placeholder}
    {type}
    {maxlength}
    aria-invalid={error ? true : undefined}
    oninput={(event) => onvalue(event.currentTarget.value)}
  />
  {#if error}<span class="error">{t(`errors.${error}`)}</span>{/if}
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

  .invalid :global(input) {
    border-color: var(--tint-red);
  }

  .error {
    font-size: 12px;
    color: var(--tint-red);
  }
</style>
