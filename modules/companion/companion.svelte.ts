import { moduleNative, toast } from '@arthex/sdk';
import { t } from './i18n';
import { ID } from './identity';
import { alreadyTried, normalized, type TriedWord } from './words';

const native = moduleNative(ID);

export type Species = 'fox' | 'dog' | 'cat' | 'wolf';
export type Mood = 'idle' | 'happy' | 'sleep' | 'sniff' | 'walk';
export type Frequency = 'rare' | 'normal' | 'often';
export type Item = 'scarf' | 'glasses' | 'bow' | 'crown';
export type FlairState = 'off' | 'loading' | 'ready';
export type GameId = 'catch' | 'memory' | 'pet' | 'flair';
export type View = 'home' | 'shop' | 'result' | GameId;

export interface Mascot {
  species: Species | null;
  name: string;
  level: number;
  xp: number;
  xpNeeded: number;
  kibble: number;
  wear: Item | null;
  owned: Item[];
  onScreen: boolean;
  frequency: Frequency;
  petsToday: number;
  petGoal: number;
}

export interface Gain {
  xp: number;
  kibble: number;
  levelUp: boolean;
}

interface Answer {
  view: Mascot;
  gain: Gain | null;
}

export interface FlairStatus {
  state: FlairState;
  percent: number;
  failed: boolean;
  bytes: number;
  words: number;
}

export type Guess =
  | { result: 'unknown' }
  | { result: 'close'; word: string; temperature: number }
  | { result: 'found'; word: string; tries: number };

export interface Result {
  title: string;
  gain: Gain;
  game: GameId;
}

const NOTHING_GAINED: Gain = { xp: 0, kibble: 0, levelUp: false };

/** Today in the user's time, as a number of days: the pets of the day start over at midnight. */
const today = (): number => {
  const now = new Date();
  return Math.floor((now.getTime() - now.getTimezoneOffset() * 60_000) / 86_400_000);
};

const reducedMotion = (): boolean => document.documentElement.dataset.motion === 'reduced';

/** What the page and the Home card share. It stays while the overlay closes, which rebuilds them. */
class Companion {
  mascot = $state<Mascot | null>(null);
  view = $state<View>('home');
  result = $state<Result | null>(null);
  flair = $state<FlairStatus | null>(null);
  /** The round in progress: the program holds its secret word for as long as it runs. */
  round = $state<TriedWord[] | null>(null);
  roundMessage = $state('');
  private listening = false;

  private async ask(method: string, params: Record<string, unknown> = {}): Promise<Gain | null> {
    try {
      const answer = await native.call<Answer>(method, { day: today(), ...params });
      this.mascot = answer.view;
      return answer.gain;
    } catch {
      toast(t('failed'));
      return null;
    }
  }

  async load(): Promise<void> {
    await this.ask('state');
    void native.call('setMotion', { reducedMotion: reducedMotion() }).catch(() => undefined);
    if (!this.listening) {
      this.listening = true;
      native.on('flair', () => void this.loadFlair());
    }
    await this.loadFlair();
  }

  async adopt(species: Species, name: string): Promise<void> {
    await this.ask('adopt', { species, name });
  }

  tap(): void {
    void this.ask('tap');
  }

  /** The last stroke of the day pays, and says so. */
  async pet(): Promise<Gain | null> {
    return this.ask('pet');
  }

  async finish(game: 'catch' | 'memory', title: string, caught = 0): Promise<void> {
    const gain = await this.ask('finish', { game, caught });
    if (gain) this.showResult(game, title, gain);
  }

  showResult(game: GameId, title: string, gain: Gain = NOTHING_GAINED): void {
    this.result = { title, gain, game };
    this.view = 'result';
  }

  /** The overlay goes away during the game, and comes back on its own at the end. */
  async hideAndSeek(): Promise<void> {
    try {
      const answer = await native.callHidden<Answer>('hideAndSeek', { day: today() });
      this.mascot = answer.view;
    } catch {
      toast(t('failed'));
    }
  }

  async buy(item: Item): Promise<void> {
    await this.ask('buy', { item });
  }

  async wear(item: Item | null): Promise<void> {
    await this.ask('wear', { item });
  }

  async saveScreen(onScreen: boolean, frequency: Frequency): Promise<void> {
    await this.ask('setScreen', { onScreen, frequency });
  }

  private async askFlair(method: string): Promise<void> {
    try {
      this.flair = await native.call<FlairStatus>(method);
      if (this.flair.failed) toast(t('flair.failed'));
    } catch {
      toast(t('failed'));
    }
  }

  loadFlair(): Promise<void> {
    return this.askFlair('flairStatus');
  }

  downloadFlair(): Promise<void> {
    return this.askFlair('flairDownload');
  }

  deleteFlair(): Promise<void> {
    return this.askFlair('flairDelete');
  }

  /** Goes on with the round in progress, if the player left one. */
  async startRound(): Promise<void> {
    if (this.round) return;
    try {
      await native.call('flairStart');
      this.round = [];
      this.roundMessage = '';
    } catch {
      toast(t('failed'));
      this.view = 'home';
    }
  }

  async guess(input: string): Promise<void> {
    const word = normalized(input);
    if (!word || !this.round) return;
    if (alreadyTried(this.round, word)) {
      this.roundMessage = t('flair.tried');
      return;
    }
    try {
      const { guess, answer } = await native.call<{ guess: Guess; answer: Answer }>('flairGuess', {
        word,
        day: today(),
      });
      this.mascot = answer.view;
      if (guess.result === 'unknown') {
        this.roundMessage = t('flair.unknown', { word });
      } else if (guess.result === 'close') {
        this.roundMessage = '';
        this.round = [...this.round, { word: guess.word, temperature: guess.temperature }];
      } else {
        this.round = null;
        this.showResult(
          'flair',
          t('flair.found', { word: guess.word, tries: guess.tries }),
          answer.gain ?? NOTHING_GAINED,
        );
      }
    } catch {
      toast(t('failed'));
    }
  }

  async giveUp(): Promise<void> {
    try {
      const { word } = await native.call<{ word: string }>('flairGiveUp');
      this.round = null;
      this.showResult('flair', t('flair.itWas', { word }));
    } catch {
      toast(t('failed'));
    }
  }
}

export const companion = new Companion();
