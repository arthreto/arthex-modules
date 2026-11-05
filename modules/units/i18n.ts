import { moduleTranslator } from '@arthex/sdk';
import type en from './locales/en.json';
import { ID } from './identity';
import type { Unit } from './units';

export const t = moduleTranslator<typeof en>(ID);

/** A value with its symbol as the active language writes it: "5 Mo" or "5 MB". */
export const withSymbol = (value: string, unit: Unit) => `${value} ${t(`units.${unit.id}.symbol`)}`;
