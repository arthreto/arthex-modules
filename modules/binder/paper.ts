import { parse, type Block, type Span } from './markdown';

/** The look of a PDF, which is always on white paper, whatever the theme of Arthex. */
const STYLE = `
  * { box-sizing: border-box; }
  body { margin: 0; font: 13.5px/1.65 'Segoe UI Variable Text', 'Segoe UI', sans-serif; color: #25282e; }
  h1, h2, h3 { color: #15171b; line-height: 1.3; break-after: avoid; }
  .title { margin: 0 0 14px; font-size: 26px; font-weight: 700; }
  h1 { margin: 12px 0 8px; font-size: 20px; }
  h2 { margin: 14px 0 4px; font-size: 16px; font-weight: 650; }
  h3 { margin: 12px 0 4px; font-size: 12.5px; letter-spacing: .06em; text-transform: uppercase; color: #646a75; }
  p { margin: 0 0 6px; }
  .item { display: flex; gap: 10px; margin: 0 0 3px; }
  .marker { flex: none; min-width: 14px; font-weight: 600; color: #0b6fb0; }
  .task-done { text-decoration: line-through; color: #646a75; }
  hr { height: 1px; margin: 12px 0; border: 0; background: #d5d8de; }
  blockquote { margin: 2px 0 8px; padding: 2px 0 2px 14px; border-left: 3px solid #d5d8de; font-style: italic; color: #646a75; }
  .callout { margin: 2px 0 8px; padding: 10px 14px; border-radius: 10px; background: #f2f3f5; border: 1px solid #d5d8de; break-inside: avoid; }
  .math { margin: 6px 0 10px; text-align: center; font-size: 19px; }
  math { font-family: 'Cambria Math', Cambria, serif; }
  pre { margin: 4px 0 10px; padding: 12px 14px; border-radius: 10px; background: #f2f3f5; border: 1px solid #d5d8de; white-space: pre-wrap; break-inside: avoid; }
  code, pre { font-family: 'Cascadia Mono', Consolas, monospace; font-size: 12.5px; }
  code { padding: 1px 5px; border-radius: 4px; background: #f2f3f5; }
  pre code { padding: 0; background: none; }
  mark { padding: 1px 3px; border-radius: 3px; background: #fdeca6; }
  a { color: #0b6fb0; }
  table { width: 100%; margin: 6px 0 12px; border-collapse: collapse; }
  th, td { padding: 7px 12px; border: 1px solid #d5d8de; text-align: left; vertical-align: top; font-size: 13px; line-height: 1.45; }
  th { font-weight: 600; color: #15171b; background: #f2f3f5; }
  tr, figure { break-inside: avoid; }
  figure { margin: 6px 0 12px; text-align: center; }
  img { max-width: 100%; max-height: 420px; border-radius: 6px; }
  figcaption { margin-top: 6px; font-size: 12.5px; color: #646a75; }
  .media { margin: 2px 0 8px; font-size: 13px; color: #646a75; }
  .file { margin: 4px 0 10px; padding: 10px 12px; border-radius: 10px; background: #f2f3f5; border: 1px solid #d5d8de; break-inside: avoid; }
  .file strong { display: block; font-weight: 500; color: #15171b; }
  .file span { font-size: 12px; color: #646a75; }
  .folded { margin: 2px 0 8px; }
  .folded > strong { font-weight: 500; color: #15171b; }
  .folded > div { margin-left: 20px; }
`;

export interface PaperTexts {
  video: (name: string) => string;
  fileSize: (file: string) => string;
  formula: (tex: string) => string;
}

const escaped = (text: string): string =>
  text
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&#39;');

const isWeb = (address: string): boolean => /^https?:\/\/[^\s"'<>]+$/.test(address);

function spanHtml(span: Span): string {
  let html = escaped(span.text);
  if (span.code) html = `<code>${html}</code>`;
  if (span.bold) html = `<strong>${html}</strong>`;
  if (span.italic) html = `<em>${html}</em>`;
  if (span.strike) html = `<s>${html}</s>`;
  if (span.mark) html = `<mark>${html}</mark>`;
  if (span.href && isWeb(span.href)) html = `<a href="${escaped(span.href)}">${html}</a>`;
  return html;
}

const spansHtml = (spans: Span[]): string => spans.map(spanHtml).join('');

const indent = (depth: number): string => (depth > 0 ? ` style="margin-left:${String(depth * 22)}px"` : '');

function blockHtml(block: Block, texts: PaperTexts): string {
  switch (block.kind) {
    case 'h1':
    case 'h2':
    case 'h3':
      return `<${block.kind}>${spansHtml(block.spans)}</${block.kind}>`;
    case 'p':
      return `<p>${spansHtml(block.spans)}</p>`;
    case 'quote':
      return `<blockquote>${spansHtml(block.spans)}</blockquote>`;
    case 'li':
      return `<div class="item"${indent(block.depth)}><span class="marker">•</span><span>${spansHtml(block.spans)}</span></div>`;
    case 'ol':
      return `<div class="item"${indent(block.depth)}><span class="marker">${escaped(block.number)}.</span><span>${spansHtml(block.spans)}</span></div>`;
    case 'task':
      return `<div class="item"><span class="marker">${block.done ? '☑' : '☐'}</span><span${block.done ? ' class="task-done"' : ''}>${spansHtml(block.spans)}</span></div>`;
    case 'callout':
      return `<div class="callout">${spansHtml(block.spans)}</div>`;
    case 'code':
      return `<pre><code>${escaped(block.code)}</code></pre>`;
    case 'math':
      return `<div class="math">${texts.formula(block.tex)}</div>`;
    case 'hr':
      return '<hr>';
    case 'gap':
      return '';
    case 'table': {
      const head = block.head.map((cell) => `<th>${spansHtml(cell)}</th>`).join('');
      const rows = block.rows
        .map((row) => `<tr>${row.map((cell) => `<td>${spansHtml(cell)}</td>`).join('')}</tr>`)
        .join('');
      return `<table><thead><tr>${head}</tr></thead><tbody>${rows}</tbody></table>`;
    }
    case 'image':
      return `<figure><img src="media/${escaped(block.file)}" alt="${escaped(block.caption)}">${
        block.caption ? `<figcaption>${escaped(block.caption)}</figcaption>` : ''
      }</figure>`;
    case 'video':
      return `<div class="media">${escaped(texts.video(block.caption || block.file))}</div>`;
    case 'file':
      return `<div class="file"><strong>${escaped(block.name)}</strong><span>${escaped(texts.fileSize(block.file))}</span></div>`;
    case 'details':
      return `<div class="folded"><strong>${spansHtml(block.summary)}</strong><div>${blocksHtml(block.body, texts)}</div></div>`;
  }
}

const blocksHtml = (blocks: Block[], texts: PaperTexts): string =>
  blocks.map((block) => blockHtml(block, texts)).join('');

/** The whole page a PDF is made from, with every sign of the user's text escaped. Sections are
 * unfolded, as paper cannot unfold them. */
export function documentHtml(title: string, text: string, texts: PaperTexts): string {
  return `<!doctype html><html><head><meta charset="utf-8"><title>${escaped(title)}</title><style>${STYLE}</style></head><body><div class="title">${escaped(title)}</div>${blocksHtml(parse(text), texts)}</body></html>`;
}
