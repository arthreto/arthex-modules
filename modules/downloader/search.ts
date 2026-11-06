import { openTool, type SearchProvider } from '@arthex/sdk';
import { t } from './i18n';
import { ICON, ID, TINT } from './identity';
import { knownSite, toLink } from './links';

let sites: Promise<ReadonlySet<string>> | undefined;

/** Thousands of names, read only once something looks like a link. */
const loadSites = () =>
  (sites ??= import('./sites.json').then(({ default: names }) => new Set<string>(names)));

export const downloaderSearch: SearchProvider = {
  order: 3,
  async search(query) {
    if (query.commandsOnly) return [];
    const link = toLink(query.text);
    if (!link) return [];
    const site = knownSite(link, await loadSites());
    if (!site) return [];
    return [
      {
        key: `${ID}:tool`,
        group: t('toolGroup'),
        kind: t('toolKind'),
        title: t('name'),
        subtitle: t('downloadFrom', { site }),
        icon: { kind: 'symbol', path: ICON, tint: TINT },
        primary: { label: t('open'), run: () => openTool(ID, link) },
      },
    ];
  },
};
