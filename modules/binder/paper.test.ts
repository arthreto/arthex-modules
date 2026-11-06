import { describe, expect, it } from 'vitest';
import { documentHtml, type PaperTexts } from './paper';

const texts: PaperTexts = {
  video: (name) => `Vidéo : ${name}`,
  fileSize: () => 'PDF · 2 Mo',
  formula: (tex) => `<math>${tex}</math>`,
};
const body = (text: string, title = 'Titre') => documentHtml(title, text, texts);

describe('documentHtml', () => {
  it('escapes everything the user wrote', () => {
    const html = body('<script>alert(1)</script>\n\n`<b>`\n\n```\n<i>\n```', '<img src=x onerror=y>');
    expect(html).not.toContain('<script>');
    expect(html).not.toContain('<b>');
    expect(html).not.toContain('<i>');
    expect(html).not.toContain('<img src=x');
    expect(html).toContain('&lt;script&gt;');
  });

  it('only links to the web', () => {
    expect(body('[a](https://a.fr)')).toContain('<a href="https://a.fr">a</a>');
    expect(body('[a](javascript:evil)')).not.toContain('href');
    expect(body('[a](file:///c:/b)')).not.toContain('href');
  });

  it('cannot leave an attribute from a caption or a file name', () => {
    const html = body('![a"onerror="x](media/b.png)');
    expect(html).toContain('alt="a&quot;onerror=&quot;x"');
  });

  it('shows an image from next to the page, but only names a video', () => {
    expect(body('![Légende](media/a.png)')).toContain('<img src="media/a.png" alt="Légende">');
    expect(body('![Légende](media/a.png)')).toContain('<figcaption>Légende</figcaption>');
    expect(body('![Cours](media/a.mp4)')).toContain('Vidéo : Cours');
    expect(body('![Cours](media/a.mp4)')).not.toContain('<video');
  });

  it('unfolds what is folded on the screen', () => {
    const html = body('<details><summary>Question</summary>\nRéponse\n</details>');
    expect(html).toContain('Question');
    expect(html).toContain('Réponse');
    expect(html).not.toContain('<details');
  });

  it('writes tables, tasks, formulas and files', () => {
    const html = body('| A |\n| --- |\n| 1 |\n\n- [x] fait\n\n$$\nx\n$$\n\n[Poly.pdf](media/a.pdf)');
    expect(html).toContain('<th>A</th>');
    expect(html).toContain('<td>1</td>');
    expect(html).toContain('☑');
    expect(html).toContain('<math>x</math>');
    expect(html).toContain('Poly.pdf');
    expect(html).toContain('PDF · 2 Mo');
  });
});
