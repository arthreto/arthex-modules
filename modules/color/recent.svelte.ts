const KEY = 'arthex.color.recent';
const MAX = 6;

function load(): string[] {
  try {
    const saved: unknown = JSON.parse(localStorage.getItem(KEY) ?? '[]');
    return Array.isArray(saved) ? saved.filter((item) => typeof item === 'string').slice(0, MAX) : [];
  } catch {
    return [];
  }
}

class Recent {
  colors = $state<string[]>(load());

  add(hex: string): void {
    this.colors = [hex, ...this.colors.filter((other) => other !== hex)].slice(0, MAX);
    try {
      localStorage.setItem(KEY, JSON.stringify(this.colors));
    } catch {
      // Only a convenience: the list simply starts empty next time.
    }
  }
}

export const recent = new Recent();
