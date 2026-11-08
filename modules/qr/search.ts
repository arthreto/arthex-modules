import { getLocale, openTool, toast, type SearchProvider, type SearchResult } from '@arthex/sdk';
import { canvasOf, copyImage, encode, PAPER, pngOf } from './code';
import { kindOfText } from './content';
import { t } from './i18n';
import { ICON, ID, TINT } from './identity';
import { createInput } from './input';

const ASKED = /^(?:qr|qrcode|qr-code)\s+(.+)$/is;
const INK = '#000000';
/** Drawn twice as large as shown, so that it stays sharp on a screen at 200 %. */
const PICTURE_PIXELS = 400;
const COPIED_PIXELS = 1024;

const icon = { kind: 'symbol', path: ICON, tint: TINT } as const;

export const qrSearch: SearchProvider = {
  order: 2,
  search(query) {
    if (query.commandsOnly) return [];
    const content = ASKED.exec(query.text)?.[1];
    const code = content ? encode(content, 'M') : null;
    if (!content || !code) return [];
    const kind = t(`kinds.${kindOfText(content)}`);
    const customize = () => openTool(ID, createInput(content));
    const pixels = COPIED_PIXELS.toLocaleString(getLocale());
    const result: SearchResult = {
      key: `${ID}:code`,
      group: t('group'),
      kind: t('kind'),
      title: content,
      subtitle: t('search.subtitle', { kind }),
      icon,
      picture: {
        src: canvasOf(code, INK, PAPER, PICTURE_PIXELS).toDataURL('image/png'),
        alt: t('codeOf', { content }),
      },
      details: [
        { label: t('search.content'), value: kind },
        { label: t('search.code'), value: t('search.codeValue', { version: code.version, size: code.size }) },
        { label: t('search.image'), value: t('search.png', { size: pixels }) },
      ],
      primary: {
        label: t('copyImage'),
        run: () =>
          copyImage(pngOf(code, INK, PAPER, COPIED_PIXELS)).then(
            () => toast(t('imageCopied')),
            () => toast(t('failed')),
          ),
      },
      secondary: { label: t('search.customize'), run: customize },
    };
    return [
      result,
      {
        key: `${ID}:tool`,
        group: t('toolGroup'),
        kind: t('toolKind'),
        title: t('name'),
        subtitle: t('search.customizeIn'),
        icon,
        primary: { label: t('open'), run: customize },
      },
    ];
  },
};
