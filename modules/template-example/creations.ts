import { moduleNative, openTool, toast } from '@arthex/sdk';
import { t } from './i18n';
import { ID } from './identity';

/** Arthex registers a module once all of them are loaded, and only then can it open its tool. */
const REGISTERED_AFTER_MS = 1000;
/** Arthex stops a program a minute after its last call: a long video must not be cut short. */
const KEEP_RUNNING_MS = 20_000;

interface Finished {
  file: string;
  folder?: string;
  reason?: string;
}

let keepRunning: ReturnType<typeof setInterval> | null = null;

const lastPart = (path: string) => path.split(/[\/]/).filter(Boolean).pop() ?? path;

/** While files are being made, asks the program how many are left, which keeps it running. */
export function creationStarted(): void {
  if (keepRunning !== null) return;
  const native = moduleNative(ID);
  keepRunning = setInterval(() => {
    native
      .call<number>('waiting')
      .then((left) => {
        if (left === 0 && keepRunning !== null) {
          clearInterval(keepRunning);
          keepRunning = null;
        }
      })
      .catch(() => undefined);
  }, KEEP_RUNNING_MS);
}

function failure({ file, reason }: Finished): string {
  if (reason === 'space') return t('failed.space', { file });
  if (reason === 'ffmpeg') return t('failed.ffmpeg');
  return t('failed.other', { file });
}

/**
 * Opens the tool after a right-click, and says when a file is made, even with the tool closed.
 * It runs while the module loads; `arthex build` loads it outside Arthex too, with nothing to do.
 */
export function watchCreations(): void {
  if (!('__ARTHEX__' in globalThis)) return;
  const native = moduleNative(ID);
  native.on('open', (payload) => openTool(ID, (payload as { folder: string }).folder));
  native.on('done', (payload) => {
    const { file, folder = '' } = payload as Finished;
    toast(t('created', { file, folder: lastPart(folder) }));
  });
  native.on('failed', (payload) => toast(failure(payload as Finished)));
  native
    .call<string | null>('takeOpen')
    .then((folder) => {
      if (folder) setTimeout(() => openTool(ID, folder), REGISTERED_AFTER_MS);
    })
    .catch(() => undefined);
}
