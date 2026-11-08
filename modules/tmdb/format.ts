/** `2024-02-27` written out in the language of the interface, or nothing. */
export function dayOf(date: string, locale: string): string {
  const parts = /^(\d{4})-(\d{2})-(\d{2})$/.exec(date);
  if (!parts) return '';
  const day = new Date(Number(parts[1]), Number(parts[2]) - 1, Number(parts[3]));
  return day.toLocaleDateString(locale, { dateStyle: 'long' });
}

/** `2 h 45`, or `48 min` under an hour. */
export function durationOf(minutes: number): string {
  if (minutes < 60) return `${minutes} min`;
  return `${Math.floor(minutes / 60)} h ${String(minutes % 60).padStart(2, '0')}`;
}

/** Whole years from a birthday to a day (today or a death). */
export function ageOf(birthday: string, until: Date): number {
  const born = new Date(`${birthday}T00:00:00`);
  const birthdayPassed =
    until.getMonth() > born.getMonth() ||
    (until.getMonth() === born.getMonth() && until.getDate() >= born.getDate());
  return until.getFullYear() - born.getFullYear() - (birthdayPassed ? 0 : 1);
}

export const ratingOf = (rating: number, locale: string) =>
  rating.toLocaleString(locale, { minimumFractionDigits: 1, maximumFractionDigits: 1 });

/** The language and the country TMDB answers for, from the interface and the system. */
export function regionOf(locale: string, systemLanguage: string): { language: string; country: string } {
  const country =
    /^[a-z]{2}-([A-Z]{2})/i.exec(systemLanguage)?.[1]?.toUpperCase() ?? (locale === 'fr' ? 'FR' : 'US');
  return { language: `${locale}-${country}`, country };
}
