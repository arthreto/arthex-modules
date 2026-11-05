import { openTool, toast, type ResultIcon, type SearchProvider, type SearchResult } from '@arthex/sdk';
import { formats, parseColor, toHex, type Color } from './color';
import { t } from './i18n';
import { ID, ICON, TINT } from './identity';
import { matchNames } from './names';

const MAX_NAMES = 6;

/** A square of the colour itself, as the result's icon. */
function swatch(color: Color): ResultIcon {
  const svg =
    `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 30 30"><rect x="1" y="1" width="28" height="28" rx="8" ` +
    `fill="rgb(${color.r},${color.g},${color.b})" fill-opacity="${color.a}" stroke="#808080" stroke-opacity=".5"/></svg>`;
  return { kind: 'image', src: `data:image/svg+xml,${encodeURIComponent(svg)}` };
}

const copyAction = (value: string) => ({
  label: t('copy'),
  run: async () => {
    await navigator.clipboard.writeText(value);
    toast(t('copied', { value }));
  },
});

function toolResult(label: string, input: string): SearchResult {
  return {
    key: `${ID}:tool`,
    group: t('toolGroup'),
    kind: t('toolKind'),
    title: t('name'),
    subtitle: t('openWith', { value: label }),
    icon: { kind: 'symbol', path: ICON, tint: TINT },
    primary: { label: t('open'), run: () => openTool(ID, input) },
  };
}

function nameResults(text: string): SearchResult[] {
  const names = matchNames(text).slice(0, MAX_NAMES);
  const first = names[0];
  if (!first) return [];
  return [
    ...names.map((name): SearchResult => {
      const color = parseColor(name.hex);
      const [hex, rgb, , hsl] = color ? formats(color) : [];
      return {
        key: `${ID}:name:${name.id}`,
        group: t('group'),
        kind: t('group'),
        title: t(`names.${name.id}`),
        subtitle: name.hex,
        icon: color ? swatch(color) : { kind: 'symbol', path: ICON, tint: TINT },
        details: [hex, rgb, hsl].flatMap((row) => (row ? [{ label: row.format, value: row.value }] : [])),
        primary: copyAction(name.hex),
      };
    }),
    toolResult(t(`names.${first.id}`), first.hex),
  ];
}

export const colorSearch: SearchProvider = {
  order: 2,
  search(query) {
    if (query.commandsOnly) return [];
    const color = parseColor(query.text);
    if (!color) return nameResults(query.text);
    const typed = query.text.trim().toLowerCase();
    return [
      ...formats(color)
        .filter((row) => row.value.toLowerCase() !== typed)
        .map((row): SearchResult => ({
          key: `${ID}:${row.format}`,
          group: t('group'),
          kind: t('group'),
          title: row.value,
          subtitle: row.format,
          icon: swatch(color),
          details: [
            { label: t('format'), value: row.format },
            { label: t('group'), value: query.text },
          ],
          primary: copyAction(row.value),
        })),
      toolResult(query.text, toHex(color)),
    ];
  },
};
