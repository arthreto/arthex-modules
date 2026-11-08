import { describe, expect, it, vi } from 'vitest';

vi.mock('@arthex/sdk', () => ({}));
vi.mock('./i18n', () => ({ t: (key: string) => key }));
vi.mock('./library', () => ({}));

const { askedOf } = await import('./search');

const words = { movie: 'film|films', tv: 'serie|series', person: 'acteur|actrice', any: 'tmdb' };

describe('search words', () => {
  it('reads the kind asked for and keeps the name as typed', () => {
    expect(askedOf('film dune', 'film Dune', words)).toEqual({ kind: 'movie', name: 'Dune' });
    expect(askedOf('serie dune prophecy', 'Série Dune Prophecy', words)).toEqual({
      kind: 'tv',
      name: 'Dune Prophecy',
    });
    expect(askedOf('tmdb chalamet', 'tmdb Chalamet', words)).toEqual({ kind: null, name: 'Chalamet' });
  });

  it('leaves other queries to the rest of the search', () => {
    expect(askedOf('dune', 'dune', words)).toBeNull();
    expect(askedOf('filmographie dune', 'filmographie dune', words)).toBeNull();
    expect(askedOf('film ', 'film', words)).toBeNull();
  });
});
