import { images, type ImageSize } from './library';

/** The posters and photos already fetched, shared by every view of the tool. */
class Pictures {
  #found = $state<Record<string, string>>({});
  #asked = new Set<string>();

  get(path: string, size: ImageSize): string {
    return this.#found[`${size}${path}`] ?? '';
  }

  /** Fetches together the images a view is about to show. */
  want(paths: string[], size: ImageSize): void {
    const missing = paths.filter((path) => path && !this.#asked.has(`${size}${path}`));
    if (missing.length === 0) return;
    for (const path of missing) this.#asked.add(`${size}${path}`);
    images(missing, size).then(
      (found) => {
        for (const [path, src] of Object.entries(found)) this.#found[`${size}${path}`] = src;
      },
      () => {
        for (const path of missing) this.#asked.delete(`${size}${path}`);
      },
    );
  }
}

export const pictures = new Pictures();
