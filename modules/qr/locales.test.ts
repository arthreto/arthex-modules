import { describe, expect, it } from 'vitest';
import en from './locales/en.json';
import fr from './locales/fr.json';

describe('locales', () => {
  // The catalog lists them in a keyed list: a word twice stops its details from opening.
  it.each([
    ['en', en],
    ['fr', fr],
  ])('gives each keyword and search example once in %s', (_, messages) => {
    const keywords = messages.keywords.split(' ');
    const examples = messages.catalog.examples.split('|').map((example) => example.trim());
    expect(new Set(keywords).size).toBe(keywords.length);
    expect(new Set(examples).size).toBe(examples.length);
  });
});
