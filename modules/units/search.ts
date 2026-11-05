import { getLocale, openTool, toast, type SearchProvider, type SearchResult } from '@arthex/sdk';
import { t, withSymbol } from './i18n';
import { ICON, ID, TINT } from './identity';
import { parseQuery } from './parse';
import { convert, formatValue, ranked, type Unit } from './units';

const MAX_RESULTS = 6;
const icon = { kind: 'symbol', path: ICON, tint: TINT } as const;

function conversionResult(source: string, value: number, unit: Unit): SearchResult {
  const locale = getLocale();
  const shown = withSymbol(formatValue(value, locale), unit);
  return {
    key: `${ID}:${unit.id}`,
    group: t('group'),
    kind: t('conversion'),
    title: shown,
    subtitle: `${source} · ${t(`units.${unit.id}.name`)}`,
    icon,
    details: [
      { label: t('conversion'), value: `${source} = ${shown}` },
      { label: t('tip'), value: t('tipText') },
    ],
    primary: {
      label: t('copy'),
      run: async () => {
        await navigator.clipboard.writeText(withSymbol(formatValue(value, locale, false), unit));
        toast(t('copied', { value: shown }));
      },
    },
  };
}

export const unitsSearch: SearchProvider = {
  order: 1,
  search(query) {
    if (query.commandsOnly) return [];
    const conversion = parseQuery(query.text);
    if (conversion === null) return [];
    const { value, from, to } = conversion;
    const source = withSymbol(formatValue(value, getLocale()), from);
    const conversions = to
      ? [{ unit: to as Unit, value: convert(value, from, to) }]
      : ranked(value, from).slice(0, MAX_RESULTS);
    return [
      ...conversions.map((result) => conversionResult(source, result.value, result.unit)),
      {
        key: `${ID}:tool`,
        group: t('toolGroup'),
        kind: t('toolKind'),
        title: t('name'),
        subtitle: t('openWith', { value: source }),
        icon,
        primary: { label: t('open'), run: () => openTool(ID, query.text) },
      },
    ];
  },
};
