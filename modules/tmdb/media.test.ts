import { describe, expect, it } from 'vitest';
import { cardOf, episodesOf, parseKey, personOf, titleOf, trailersOf, type RawTitle } from './media';

const video = (name: string, type: string, language: string, official = true) => ({
  key: name,
  name,
  site: 'YouTube',
  type,
  official,
  iso_639_1: language,
});

const dune: RawTitle = {
  id: 693134,
  title: 'Dune : Deuxième partie',
  release_date: '2024-02-27',
  poster_path: '/poster.jpg',
  vote_average: 8.144,
  vote_count: 8923,
  runtime: 165,
  genres: [{ name: 'Science-Fiction' }],
  overview: 'Paul Atreides…',
  credits: {
    cast: [{ id: 1190668, name: 'Timothée Chalamet', character: 'Paul Atreides', profile_path: '/tc.jpg' }],
    crew: [
      { id: 137427, name: 'Denis Villeneuve', job: 'Director' },
      { id: 1, name: 'Someone', job: 'Producer' },
    ],
  },
  videos: {
    results: [
      video('Clip', 'Clip', 'en'),
      video('Teaser', 'Teaser', 'fr'),
      video('Official Trailer', 'Trailer', 'en'),
      video('Bande-annonce VF', 'Trailer', 'fr'),
      { ...video('Vimeo', 'Trailer', 'fr'), site: 'Vimeo' },
    ],
  },
  recommendations: {
    results: [{ id: 438631, title: 'Dune', release_date: '2021-09-15', poster_path: '/d.jpg' }],
  },
  'watch/providers': {
    results: { FR: { rent: [{ provider_name: 'Canal VOD', logo_path: '/canal.png' }] } },
  },
  release_dates: {
    results: [{ iso_3166_1: 'FR', release_dates: [{ certification: '' }, { certification: '12' }] }],
  },
};

describe('media', () => {
  it('turns search results into cards and leaves out what cannot be opened', () => {
    expect(
      cardOf({
        id: 505710,
        media_type: 'person',
        name: 'Zendaya',
        profile_path: '/z.jpg',
        known_for_department: 'Acting',
      }),
    ).toEqual({
      key: 'person:505710',
      kind: 'person',
      title: 'Zendaya',
      year: '',
      rating: 0,
      image: '/z.jpg',
      department: 'Acting',
    });
    expect(cardOf({ id: 1, media_type: 'collection', name: 'Dune' })).toBeNull();
    expect(cardOf({ id: 90228, name: 'Dune : Prophecy', first_air_date: '2024-11-17' }, 'tv')?.year).toBe(
      '2024',
    );
  });

  it('puts trailers first, in the language asked, and keeps only YouTube', () => {
    expect(trailersOf(dune, 'fr').map((trailer) => trailer.name)).toEqual([
      'Bande-annonce VF',
      'Official Trailer',
      'Teaser',
    ]);
    expect(trailersOf(dune, 'fr')[0]?.address).toBe('https://www.youtube.com/watch?v=Bande-annonce VF');
  });

  it('reads a film with its director, cast, providers and rating for the country', () => {
    const title = titleOf(dune, 'movie', 'fr', 'FR');
    expect(title).toMatchObject({
      key: 'movie:693134',
      year: '2024',
      runtime: 165,
      certification: '12',
      rating: 8.1,
      makers: [{ key: 'person:137427', name: 'Denis Villeneuve' }],
      providers: { stream: [], rent: [{ name: 'Canal VOD', logo: '/canal.png' }], buy: [] },
    });
    expect(title.cast[0]).toMatchObject({ key: 'person:1190668', role: 'Paul Atreides' });
    expect(title.recommendations[0]?.key).toBe('movie:438631');
    expect(titleOf(dune, 'movie', 'fr', 'US').certification).toBe('');
  });

  it('reads a series with its creators, roles across seasons and seasons without specials', () => {
    const series = titleOf(
      {
        id: 90228,
        name: 'Dune : Prophecy',
        created_by: [{ id: 5, name: 'Diane Ademu-John' }],
        aggregate_credits: {
          cast: [{ id: 1639, name: 'Emily Watson', roles: [{ character: 'Valya Harkonnen' }] }],
        },
        seasons: [{ season_number: 0 }, { season_number: 1 }, { season_number: 2 }],
        content_ratings: { results: [{ iso_3166_1: 'FR', rating: '12' }] },
      },
      'tv',
      'fr',
      'FR',
    );
    expect(series.makers).toEqual([{ key: 'person:5', name: 'Diane Ademu-John' }]);
    expect(series.cast[0]?.role).toBe('Valya Harkonnen');
    expect(series.seasons).toEqual([1, 2]);
    expect(series.certification).toBe('12');
  });

  it('reads episodes', () => {
    expect(episodesOf({ episodes: [{ episode_number: 1, name: 'Pilot', still_path: null }] })).toEqual([
      { number: 1, title: 'Pilot', date: '', runtime: 0, overview: '', image: '' },
    ]);
  });

  it('lists a filmography once per title, newest first, with what made the person known', () => {
    const person = personOf({
      id: 505710,
      name: 'Zendaya',
      combined_credits: {
        cast: [
          {
            id: 1,
            media_type: 'movie',
            title: 'Old',
            release_date: '2017-07-05',
            character: 'Michelle',
            poster_path: '/o.jpg',
            vote_count: 20000,
          },
          {
            id: 2,
            media_type: 'movie',
            title: 'New',
            release_date: '2024-02-27',
            character: 'Chani',
            poster_path: '/n.jpg',
            vote_count: 9000,
          },
          {
            id: 2,
            media_type: 'movie',
            title: 'New',
            release_date: '2024-02-27',
            character: 'Chani (voice)',
            poster_path: '/n.jpg',
          },
          { id: 3, media_type: 'tv', name: 'Talk show', first_air_date: '2020-01-01', character: 'Self' },
        ],
        crew: [
          { id: 4, media_type: 'movie', title: 'Directed', release_date: '2025-01-01', job: 'Director' },
        ],
      },
    });
    expect(person.credits.map((credit) => `${credit.year} ${credit.title} ${credit.role}`)).toEqual([
      '2025 Directed Director',
      '2024 New Chani',
      '2017 Old Michelle',
    ]);
    expect(person.knownFor.map((card) => card.title)).toEqual(['Old', 'New']);
  });

  it('reads the keys of pages', () => {
    expect(parseKey('tv:90228')).toEqual({ kind: 'tv', id: 90228 });
    expect(parseKey('movie:')).toBeNull();
    expect(parseKey('collection:1')).toBeNull();
  });
});
