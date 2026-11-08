import { openTool, toast, type SearchProvider, type SearchResult } from '@arthex/sdk';
import { algorithm, PASSWORD_NAMES, USUAL, type AlgorithmId } from './algorithms';
import { base64, bytesOfHex } from './formats';
import { digestsOf } from './hash.svelte';
import { t } from './i18n';
import { ICON, ID, TINT } from './identity';
import { compareInput, textInput, verifyInput } from './input';
import { algorithmsOfBits, parseQuery } from './parse';

const icon = { kind: 'symbol', path: ICON, tint: TINT } as const;

const nameOf = (id: AlgorithmId) => algorithm(id)?.name ?? id;

function digestResult(id: AlgorithmId, hex: string, text: string): SearchResult {
  const name = nameOf(id);
  return {
    key: `${ID}:${id}`,
    group: t('group'),
    kind: t('kind'),
    title: hex,
    subtitle: t('search.of', { name, text }),
    icon,
    details: [
      { label: t('search.algorithm'), value: name },
      { label: t('search.text'), value: text },
      { label: 'Base64', value: base64(bytesOfHex(hex)) },
    ],
    primary: {
      label: t('copy'),
      run: async () => {
        await navigator.clipboard.writeText(hex);
        toast(t('copied', { name }));
      },
    },
  };
}

function toolResult(subtitle: string, input: string, group = t('toolGroup')): SearchResult {
  return {
    key: `${ID}:tool`,
    group,
    kind: t('toolKind'),
    title: t('name'),
    subtitle,
    icon,
    primary: { label: t('open'), run: () => openTool(ID, input) },
  };
}

export const hashSearch: SearchProvider = {
  order: 2,
  async search(query) {
    if (query.commandsOnly) return [];
    const asked = parseQuery(query.text, t('allWords').split('|'));
    if (!asked) return [];
    if (asked.kind === 'digest') {
      const names = algorithmsOfBits(asked.bits).map(nameOf).join(', ');
      return [
        {
          ...toolResult(t('search.digestOf', { names }), compareInput(asked.hex), t('group')),
          title: t('search.digest', { bits: asked.bits }),
          details: [{ label: t('search.algorithms'), value: names }],
        },
      ];
    }
    if (asked.kind === 'password') {
      return [
        {
          ...toolResult(t('search.checkPassword'), verifyInput(asked.stored), t('group')),
          title: t('search.passwordHash', { name: PASSWORD_NAMES[asked.algorithm] }),
        },
      ];
    }
    const wanted: readonly AlgorithmId[] = asked.kind === 'one' ? [asked.algorithm] : USUAL;
    const digests = await digestsOf(asked.text);
    return [
      ...wanted.flatMap((id) => {
        const digest = digests.find((found) => found.id === id);
        return digest ? [digestResult(id, digest.hex, asked.text)] : [];
      }),
      toolResult(t('search.allOf', { text: asked.text }), textInput(asked.text)),
    ];
  },
};
