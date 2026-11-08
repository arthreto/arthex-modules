/** What TMDB answers, in the parts the module reads. */

export type MediaKind = 'movie' | 'tv' | 'person';

export interface RawResult {
  id: number;
  media_type?: string;
  title?: string;
  name?: string;
  release_date?: string;
  first_air_date?: string;
  poster_path?: string | null;
  profile_path?: string | null;
  vote_average?: number;
  vote_count?: number;
  known_for_department?: string;
  character?: string;
  job?: string;
  episode_count?: number;
  roles?: { character?: string }[];
}

interface RawVideo {
  key: string;
  name: string;
  site: string;
  type: string;
  official: boolean;
  iso_639_1: string;
}

interface RawProvider {
  provider_name: string;
  logo_path: string;
}

export interface RawTitle extends RawResult {
  overview?: string;
  runtime?: number;
  genres?: { name: string }[];
  status?: string;
  number_of_seasons?: number;
  number_of_episodes?: number;
  seasons?: { season_number: number }[];
  created_by?: RawResult[];
  credits?: { cast: RawResult[]; crew: RawResult[] };
  aggregate_credits?: { cast: RawResult[] };
  videos?: { results: RawVideo[] };
  recommendations?: { results: RawResult[] };
  'watch/providers'?: {
    results: Record<string, { flatrate?: RawProvider[]; rent?: RawProvider[]; buy?: RawProvider[] }>;
  };
  release_dates?: { results: { iso_3166_1: string; release_dates: { certification: string }[] }[] };
  content_ratings?: { results: { iso_3166_1: string; rating: string }[] };
}

export interface RawSeason {
  episodes: {
    episode_number: number;
    name: string;
    air_date?: string | null;
    runtime?: number | null;
    overview?: string;
    still_path?: string | null;
  }[];
}

export interface RawPerson extends RawResult {
  biography?: string;
  birthday?: string | null;
  deathday?: string | null;
  place_of_birth?: string | null;
  gender?: number;
  combined_credits?: { cast: RawResult[]; crew: RawResult[] };
}

/** `movie:693134`, `tv:90228` or `person:505710`: what the tool opens. */
export type MediaKey = `${MediaKind}:${number}`;

export interface Card {
  key: MediaKey;
  kind: MediaKind;
  title: string;
  year: string;
  rating: number;
  /** The poster of a title or the photo of a person, as TMDB names it. */
  image: string;
  department: string;
}

export interface Trailer {
  name: string;
  address: string;
}

export interface Provider {
  name: string;
  logo: string;
}

export interface Title {
  key: MediaKey;
  kind: 'movie' | 'tv';
  title: string;
  date: string;
  year: string;
  runtime: number;
  certification: string;
  genres: string[];
  rating: number;
  votes: number;
  overview: string;
  image: string;
  /** Directors of a film, creators of a series. */
  makers: { key: MediaKey; name: string }[];
  trailers: Trailer[];
  cast: (Card & { role: string })[];
  providers: { stream: Provider[]; rent: Provider[]; buy: Provider[] };
  recommendations: Card[];
  seasons: number[];
  seasonCount: number;
  episodeCount: number;
  status: string;
}

export interface Episode {
  number: number;
  title: string;
  date: string;
  runtime: number;
  overview: string;
  image: string;
}

export interface Person {
  key: MediaKey;
  name: string;
  department: string;
  gender: number;
  birthday: string;
  deathday: string;
  place: string;
  biography: string;
  image: string;
  knownFor: Card[];
  credits: { key: MediaKey; kind: 'movie' | 'tv'; year: string; title: string; role: string }[];
}

const CAST_SHOWN = 12;
const KNOWN_FOR = 8;

const yearOf = (date?: string | null) => (date ?? '').slice(0, 4);
const isSelf = (role?: string) => /^(self|himself|herself|lui-même|elle-même)\b/i.test(role ?? '');

/** A result of a search or a list as a card; `null` for what the tool cannot open. */
export function cardOf(raw: RawResult, kind = raw.media_type): Card | null {
  if (kind !== 'movie' && kind !== 'tv' && kind !== 'person') return null;
  return {
    key: `${kind}:${raw.id}`,
    kind,
    title: raw.title ?? raw.name ?? '',
    year: yearOf(raw.release_date ?? raw.first_air_date),
    rating: Math.round((raw.vote_average ?? 0) * 10) / 10,
    image: (kind === 'person' ? raw.profile_path : raw.poster_path) ?? '',
    department: raw.known_for_department ?? '',
  };
}

const cards = (results: RawResult[] | undefined, kind?: MediaKind) =>
  (results ?? []).flatMap((raw) => cardOf(raw, kind ?? raw.media_type) ?? []);

/** Trailers before teasers, in the language of the interface first, official ones first. */
export function trailersOf(raw: RawTitle, language: string): Trailer[] {
  return (raw.videos?.results ?? [])
    .filter((video) => video.site === 'YouTube' && (video.type === 'Trailer' || video.type === 'Teaser'))
    .sort(
      (a, b) =>
        Number(a.type !== 'Trailer') - Number(b.type !== 'Trailer') ||
        Number(a.iso_639_1 !== language) - Number(b.iso_639_1 !== language) ||
        Number(!a.official) - Number(!b.official),
    )
    .map((video) => ({ name: video.name, address: `https://www.youtube.com/watch?v=${video.key}` }));
}

function certificationOf(raw: RawTitle, country: string): string {
  if (raw.content_ratings) {
    return raw.content_ratings.results.find((entry) => entry.iso_3166_1 === country)?.rating ?? '';
  }
  const dates = raw.release_dates?.results.find((entry) => entry.iso_3166_1 === country)?.release_dates ?? [];
  return dates.find((date) => date.certification)?.certification ?? '';
}

export function titleOf(raw: RawTitle, kind: 'movie' | 'tv', language: string, country: string): Title {
  const watch = raw['watch/providers']?.results[country] ?? {};
  const providers = (list?: RawProvider[]) =>
    (list ?? []).map((provider) => ({ name: provider.provider_name, logo: provider.logo_path }));
  const castList = kind === 'tv' ? (raw.aggregate_credits?.cast ?? raw.credits?.cast) : raw.credits?.cast;
  const makers =
    kind === 'tv'
      ? (raw.created_by ?? [])
      : (raw.credits?.crew ?? []).filter((member) => member.job === 'Director');
  return {
    key: `${kind}:${raw.id}`,
    kind,
    title: raw.title ?? raw.name ?? '',
    date: (kind === 'movie' ? raw.release_date : raw.first_air_date) ?? '',
    year: yearOf(raw.release_date ?? raw.first_air_date),
    runtime: raw.runtime ?? 0,
    certification: certificationOf(raw, country),
    genres: (raw.genres ?? []).map((genre) => genre.name),
    rating: Math.round((raw.vote_average ?? 0) * 10) / 10,
    votes: raw.vote_count ?? 0,
    overview: raw.overview ?? '',
    image: raw.poster_path ?? '',
    makers: makers.map((maker) => ({ key: `person:${maker.id}`, name: maker.name ?? '' })),
    trailers: trailersOf(raw, language),
    cast: (castList ?? []).slice(0, CAST_SHOWN).flatMap((member) => {
      const card = cardOf(member, 'person');
      const role = member.character ?? member.roles?.[0]?.character ?? '';
      return card ? [{ ...card, role }] : [];
    }),
    providers: { stream: providers(watch.flatrate), rent: providers(watch.rent), buy: providers(watch.buy) },
    recommendations: cards(raw.recommendations?.results, kind),
    seasons: (raw.seasons ?? []).map((season) => season.season_number).filter((number) => number > 0),
    seasonCount: raw.number_of_seasons ?? 0,
    episodeCount: raw.number_of_episodes ?? 0,
    status: raw.status ?? '',
  };
}

export function episodesOf(raw: RawSeason): Episode[] {
  return raw.episodes.map((episode) => ({
    number: episode.episode_number,
    title: episode.name,
    date: episode.air_date ?? '',
    runtime: episode.runtime ?? 0,
    overview: episode.overview ?? '',
    image: episode.still_path ?? '',
  }));
}

export function personOf(raw: RawPerson): Person {
  const cast = (raw.combined_credits?.cast ?? []).filter(
    (credit) => !(credit.media_type === 'tv' && isSelf(credit.character)),
  );
  const directed = (raw.combined_credits?.crew ?? [])
    .filter((credit) => credit.job === 'Director')
    .map((credit) => ({ ...credit, character: credit.job }));
  const seen = new Set<string>();
  const credits = [...cast, ...directed]
    .sort((a, b) =>
      (b.release_date ?? b.first_air_date ?? '').localeCompare(a.release_date ?? a.first_air_date ?? ''),
    )
    .flatMap((credit) => {
      const card = cardOf(credit);
      if (!card || card.kind === 'person' || seen.has(card.key)) return [];
      seen.add(card.key);
      return [
        { key: card.key, kind: card.kind, year: card.year, title: card.title, role: credit.character ?? '' },
      ];
    });
  const knownFor = [
    ...new Map(
      [...cast]
        .filter((credit) => credit.poster_path && !isSelf(credit.character))
        .sort((a, b) => (b.vote_count ?? 0) - (a.vote_count ?? 0))
        .flatMap((credit) => {
          const card = cardOf(credit);
          return card ? [[card.key, card] as const] : [];
        }),
    ).values(),
  ].slice(0, KNOWN_FOR);
  return {
    key: `person:${raw.id}`,
    name: raw.name ?? '',
    department: raw.known_for_department ?? '',
    gender: raw.gender ?? 0,
    birthday: raw.birthday ?? '',
    deathday: raw.deathday ?? '',
    place: raw.place_of_birth ?? '',
    biography: raw.biography ?? '',
    image: raw.profile_path ?? '',
    knownFor,
    credits,
  };
}

export function parseKey(key: string): { kind: MediaKind; id: number } | null {
  const match = /^(movie|tv|person):(\d+)$/.exec(key);
  if (!match?.[1] || !match[2]) return null;
  return { kind: match[1] as MediaKind, id: Number(match[2]) };
}
