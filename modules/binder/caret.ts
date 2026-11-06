/** What a textarea cannot say by itself, measured on a hidden copy of its text. */

const COPIED = [
  'boxSizing',
  'width',
  'paddingTop',
  'paddingRight',
  'paddingBottom',
  'paddingLeft',
  'borderTopWidth',
  'borderRightWidth',
  'borderBottomWidth',
  'borderLeftWidth',
  'borderStyle',
  'fontFamily',
  'fontSize',
  'fontWeight',
  'lineHeight',
  'letterSpacing',
  'tabSize',
] as const;

/** Where the caret is inside the textarea, in pixels from its corner, under its line. */
export function caretPosition(area: HTMLTextAreaElement): { left: number; top: number } {
  const style = getComputedStyle(area);
  const copy = document.createElement('div');
  for (const property of COPIED) copy.style[property] = style[property];
  copy.style.position = 'absolute';
  copy.style.visibility = 'hidden';
  copy.style.whiteSpace = 'pre-wrap';
  copy.style.overflowWrap = 'break-word';
  copy.textContent = area.value.slice(0, area.selectionStart);
  const mark = document.createElement('span');
  mark.textContent = '​';
  copy.append(mark);
  area.parentElement?.append(copy);
  const top = mark.offsetTop + mark.offsetHeight - area.scrollTop;
  const left = mark.offsetLeft - area.scrollLeft;
  copy.remove();
  return { left, top };
}
