import { matchesAll, navigate, type SearchProvider, type SearchResult } from '@arthex/sdk';
import type { Connection } from './connection';
import { t } from './i18n';
import { ICON, ID, TINT } from './identity';
import { placeOf, remote } from './remote.svelte';

function result(connection: Connection): SearchResult {
  const on = remote.statusOf(connection.id).state === 'on';
  const place = placeOf(connection);
  const protocol = connection.protocol.toUpperCase();
  const server = connection.auth === 'anonymous' ? connection.host : `${connection.user}@${connection.host}`;
  const state = on ? (place ? t('status.on', { place }) : t('status.onFolder')) : t('status.off');
  return {
    key: `${ID}:${connection.id}`,
    group: t('search.group'),
    kind: t('search.kind'),
    title: connection.name,
    subtitle: `${state} · ${protocol} · ${server}`,
    icon: { kind: 'symbol', path: ICON, tint: TINT },
    details: [
      { label: t('search.protocol'), value: protocol },
      { label: t('search.server'), value: server },
      { label: t('search.drive'), value: place || t('noLetter') },
    ],
    primary: on
      ? { label: t('open', { place: place || connection.name }), run: () => remote.open(connection) }
      : { label: t('connect'), run: () => remote.connect(connection, true) },
    secondary: on
      ? { label: t('disconnect'), run: () => remote.disconnect(connection) }
      : {
          label: t('search.edit'),
          run: () => {
            remote.select(connection.id);
            navigate(ID);
          },
        },
  };
}

export const remoteSearch: SearchProvider = {
  order: 4,
  async search(query) {
    if (query.commandsOnly) return [];
    await remote.ready();
    return remote.connections
      .filter((connection) => matchesAll(`${connection.name} ${connection.host}`, query.text))
      .filter((connection) => remote.canConnect(connection) || remote.statusOf(connection.id).state === 'on')
      .map(result);
  },
};
