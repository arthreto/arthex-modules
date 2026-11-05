<script lang="ts">
  import { t } from './i18n';

  interface Props {
    hue: number;
    saturation: number;
    value: number;
    /** The colour under the marker, as CSS. */
    color: string;
    onpick: (hue: number, saturation: number) => void;
  }

  let { hue, saturation, value, color, onpick }: Props = $props();

  const HUE_STEP = 5;
  const SATURATION_STEP = 0.05;

  const angle = $derived((hue * Math.PI) / 180);
  const left = $derived(50 + 50 * saturation * Math.sin(angle));
  const top = $derived(50 - 50 * saturation * Math.cos(angle));

  function pickAt(event: PointerEvent) {
    const box = (event.currentTarget as HTMLElement).getBoundingClientRect();
    const dx = event.clientX - box.left - box.width / 2;
    const dy = event.clientY - box.top - box.height / 2;
    const degrees = ((Math.atan2(dx, -dy) * 180) / Math.PI + 360) % 360;
    onpick(degrees, Math.min(1, Math.hypot(dx, dy) / (box.width / 2)));
  }

  function onpointerdown(event: PointerEvent) {
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
    pickAt(event);
  }

  function onpointermove(event: PointerEvent) {
    if ((event.currentTarget as HTMLElement).hasPointerCapture(event.pointerId)) pickAt(event);
  }

  function onkeydown(event: KeyboardEvent) {
    const clampUnit = (next: number) => Math.min(1, Math.max(0, next));
    if (event.key === 'ArrowLeft') onpick((hue - HUE_STEP + 360) % 360, saturation);
    else if (event.key === 'ArrowRight') onpick((hue + HUE_STEP) % 360, saturation);
    else if (event.key === 'ArrowUp') onpick(hue, clampUnit(saturation + SATURATION_STEP));
    else if (event.key === 'ArrowDown') onpick(hue, clampUnit(saturation - SATURATION_STEP));
    else return;
    event.preventDefault();
  }
</script>

<button type="button" class="wheel" aria-label={t('wheel')} {onpointerdown} {onpointermove} {onkeydown}>
  <span class="dim" style:opacity={1 - value}></span>
  <span class="marker" style:left="{left}%" style:top="{top}%" style:background={color}></span>
</button>

<style>
  .wheel {
    position: relative;
    display: block;
    flex: none;
    align-self: center;
    width: 272px;
    height: 272px;
    border-radius: 50%;
    cursor: crosshair;
    touch-action: none;
    background:
      radial-gradient(closest-side, #ffffff, rgba(255, 255, 255, 0)),
      conic-gradient(#ff0000, #ffff00, #00ff00, #00ffff, #0000ff, #ff00ff, #ff0000);
  }

  .wheel:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 3px;
  }

  .dim {
    position: absolute;
    inset: 0;
    border-radius: 50%;
    background: #000000;
  }

  .marker {
    position: absolute;
    width: 16px;
    height: 16px;
    transform: translate(-50%, -50%);
    border-radius: 50%;
    border: 2px solid #ffffff;
    box-shadow: 0 0 0 1px rgba(0, 0, 0, 0.5);
  }
</style>
