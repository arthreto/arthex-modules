import { getLocale, openTool, toast, type SearchProvider, type SearchResult } from '@arthex/sdk';
import { ratingOf } from './format';
import { t } from './i18n';
import { ICON, ID, TINT } from './identity';
import { searchInput } from './input';
import { departmentOf } from './labels';
import { hasKey, images, openAddress, problemOf, search, title } from './library';
import type { Card, MediaKind } from './media';

const SHOWN = 5;

const icon = { kind: 'symbol', path: ICON, tint: TINT } as const;

/** `film dune`: the kind asked for and the name, or nothing when the query is not for TMDB. */
export function askedOf(
  normalized: string,
  text: string,
  words: Record<MediaKind | 'any', string>,
): { kind: MediaKind | null; name: string } | null {
  const match = /^(\S+)\s+(.+)$/s.exec(normalized);
  const name = /^\S+\s+(.+)$/s.exec(text)?.[1]?.trim();
  if (!match?.[1] || !name) return null;
  const first = match[1];
  const kinds = ['movie', 'tv', 'person', 'any'] as const;
  const kind = kinds.find((candidate) => words[candidate].split('|').includes(first));
  if (!kind) return null;
  return { kind: kind === 'any' ? null : kind, name };
}

async function playTrailer(card: Card) {
  if (card.kind === 'person') return;
  const found = await title(card.kind, Number(card.key.split(':')[1]));
  const trailer = found.trailers[0];
  if (trailer) await openAddress(trailer.address);
  else toast(t('noTrailer'));
}

function resultOf(card: Card, image: string): SearchResult {
  const locale = getLocale();
  const isPerson = card.kind === 'person';
  return {
    key: `${ID}:${card.key}`,
    group: t('group'),
    kind: t(`kinds.${card.kind}`),
    title: card.title,
    subtitle: isPerson
      ? departmentOf(card.department)
      : [t(`kinds.${card.kind}`), card.year, card.rating ? `★ ${ratingOf(card.rating, locale)}` : '']
          .filter(Boolean)
          .join(' · '),
    icon: image ? { kind: 'image', src: image } : icon,
    details: isPerson
      ? [{ label: t('search.department'), value: departmentOf(card.department) }]
      : [
          { label: t('search.year'), value: card.year || '—' },
          {
            label: t('search.rating'),
            value: card.rating ? t('outOfTen', { rating: ratingOf(card.rating, locale) }) : '—',
          },
        ],
    primary: { label: t('search.open'), run: () => openTool(ID, card.key) },
    ...(!isPerson && {
      secondary: { label: t('trailer'), run: () => playTrailer(card).catch(() => toast(t('failed'))) },
    }),
  };
}

export const tmdbSearch: SearchProvider = {
  order: 4,
  async search(query) {
    if (query.commandsOnly) return [];
    const words = {
      movie: t('searchWords.movie'),
      tv: t('searchWords.tv'),
      person: t('searchWords.person'),
      any: t('searchWords.any'),
    };
    const asked = askedOf(query.normalized, query.text, words);
    if (!asked) return [];
    const toolResult = (subtitle: string, input: string, title = t('name')): SearchResult => ({
      key: `${ID}:tool`,
      group: t('toolGroup'),
      kind: t('toolKind'),
      title,
      subtitle,
      icon,
      primary: { label: t('search.open'), run: () => openTool(ID, input) },
    });
    if (!(await hasKey())) {
      return [
        {
          ...toolResult(t('search.addKeyDetail'), '', t('search.addKey')),
          group: t('group'),
          details: [{ label: t('search.opening'), value: t('search.openingValue') }],
        },
      ];
    }
    let found: Card[];
    try {
      found = (await search(asked.name))
        .filter((card) => !asked.kind || card.kind === asked.kind)
        .slice(0, SHOWN);
    } catch (error) {
      if (problemOf(error) === 'refused') return [toolResult(t('key.refused'), '', t('search.addKey'))];
      return [];
    }
    const pictures = await images(
      found.map((card) => card.image),
      'w92',
    ).catch(() => ({}) as Record<string, string>);
    return [
      ...found.map((card) => resultOf(card, pictures[card.image] ?? '')),
      toolResult(t('search.searchIn', { query: asked.name }), searchInput(asked.name)),
    ];
  },
};
