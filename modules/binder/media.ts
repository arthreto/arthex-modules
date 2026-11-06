/** Arthex serves what the program copied next to the module's own files. */
const BASE = import.meta.url.slice(0, import.meta.url.lastIndexOf('/') + 1);

export const mediaUrl = (file: string): string => `${BASE}user-media/${encodeURIComponent(file)}`;
