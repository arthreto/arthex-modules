import { getLocale, moduleNative } from '@arthex/sdk';
import { regionOf } from './format';
import { ID } from './identity';
import {
  cardOf,
  episodesOf,
  personOf,
  titleOf,
  type Card,
  type Episode,
  type Person,
  type RawPerson,
  type RawResult,
  type RawSeason,
  type RawTitle,
  type Title,
} from './media';

export type Problem = 'no-key' | 'refused' | 'offline' | 'failed';
export type ImageSize = 'w92' | 'w185' | 'w300' | 'w342';

const native = moduleNative(ID);

export function problemOf(error: unknown): Problem {
  const message = String(error);
  if (message.includes('no-key')) return 'no-key';
  if (message.includes('refused')) return 'refused';
  return message.includes('offline') ? 'offline' : 'failed';
}

const region = () => regionOf(getLocale(), navigator.language);

function get<T>(path: string, params: Record<string, string> = {}): Promise<T> {
  return native.call<T>('get', { path, params: { language: region().language, ...params } });
}

export const hasKey = () => native.call<boolean>('hasKey');
export const saveKey = (key: string) => native.call('saveKey', { key });
export const removeKey = () => native.call('removeKey');
/** A YouTube or TMDB page, in the browser. */
export const openAddress = (address: string) => native.call('open', { address });

/** Images as `data:` addresses keyed by their TMDB path; the missing ones are left out. */
export function images(paths: string[], size: ImageSize): Promise<Record<string, string>> {
  const wanted = [...new Set(paths.filter(Boolean))];
  return wanted.length > 0 ? native.call('images', { paths: wanted, size }) : Promise.resolve({});
}

export async function search(query: string): Promise<Card[]> {
  const answer = await get<{ results: RawResult[] }>('/search/multi', { query });
  return answer.results.flatMap((raw) => cardOf(raw) ?? []);
}

export async function trending(): Promise<Card[]> {
  const answer = await get<{ results: RawResult[] }>('/trending/all/week');
  return answer.results.flatMap((raw) => cardOf(raw) ?? []);
}

const titles = new Map<string, Promise<Title>>();
const people = new Map<string, Promise<Person>>();

/** Each page is asked once a session: going back shows it at once. */
function remembered<T>(cache: Map<string, Promise<T>>, key: string, load: () => Promise<T>): Promise<T> {
  const known = cache.get(key);
  if (known) return known;
  const loading = load();
  cache.set(key, loading);
  loading.catch(() => cache.delete(key));
  return loading;
}

export function title(kind: 'movie' | 'tv', id: number): Promise<Title> {
  const { language, country } = region();
  return remembered(titles, `${language}${kind}${id}`, async () => {
    const append =
      kind === 'movie'
        ? 'credits,videos,recommendations,watch/providers,release_dates'
        : 'aggregate_credits,videos,recommendations,watch/providers,content_ratings';
    const raw = await get<RawTitle>(`/${kind}/${id}`, {
      append_to_response: append,
      include_video_language: `${getLocale()},en,null`,
    });
    return titleOf(raw, kind, getLocale(), country);
  });
}

export async function season(id: number, number: number): Promise<Episode[]> {
  return episodesOf(await get<RawSeason>(`/tv/${id}/season/${number}`));
}

export function person(id: number): Promise<Person> {
  return remembered(people, `${region().language}${id}`, async () => {
    const found = personOf(await get<RawPerson>(`/person/${id}`, { append_to_response: 'combined_credits' }));
    if (found.biography || getLocale() === 'en') return found;
    // Many biographies exist only in English: better than none.
    const english = await get<RawPerson>(`/person/${id}`, { language: 'en-US' });
    return { ...found, biography: english.biography ?? '' };
  });
}
