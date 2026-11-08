import { moduleNative, openTool } from '@arthex/sdk';
import { ID } from './identity';
import { OPEN_FILE } from './input';

/** Arthex registers a module once all of them are loaded, and only then can it open its tool. */
const REGISTERED_AFTER_MS = 1000;

/**
 * Opens the tool on the file after a right-click. It runs while the module loads, so that it works
 * with the tool closed. `arthex build` loads the module outside Arthex too, where there is nothing to do.
 */
export function openWhenAsked(): void {
  if (!('__ARTHEX__' in globalThis)) return;
  const native = moduleNative(ID);
  native.on('open', () => openTool(ID, OPEN_FILE));
  native
    .call<boolean>('takeOpen')
    .then((asked) => {
      if (asked) setTimeout(() => openTool(ID, OPEN_FILE), REGISTERED_AFTER_MS);
    })
    .catch(() => undefined);
}
