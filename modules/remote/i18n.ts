import { moduleTranslator } from '@arthex/sdk';
import type en from './locales/en.json';
import { ID } from './identity';

export const t = moduleTranslator<typeof en>(ID);
