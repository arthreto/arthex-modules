import { getLocale, moduleNative, toast } from '@arthex/sdk';
import { languageOf, type Format, type Language, type Speed } from './choices';
import { t } from './i18n';
import { ID } from './identity';
import en from './locales/en.json';
import fr from './locales/fr.json';

const POLL_MS = 400;
const native = moduleNative(ID);
const SAMPLES: Record<Language, string> = { fr: fr.sample, en: en.sample };

interface ModelStatus {
  state: 'missing' | 'downloading' | 'ready';
  percent: number;
  failed: boolean;
  bytes: number;
}

interface Status {
  model: ModelStatus;
  speaking: boolean;
  percent: number;
  failed: boolean;
  seconds: number | null;
}

function bytesOf(base64: string): ArrayBuffer {
  const text = atob(base64);
  const bytes = new Uint8Array(text.length);
  for (let index = 0; index < text.length; index += 1) bytes[index] = text.charCodeAt(index);
  return bytes.buffer;
}

/** Lives with the module, not the tool: a download or a voice goes on when the tool is closed. */
class Tts {
  status = $state<Status | null>(null);
  language = $state<Language>(languageOf(getLocale()));
  text = $state(SAMPLES[languageOf(getLocale())]);
  voice = $state(0);
  speed = $state<Speed>(1);
  format = $state<Format>('wav');
  duration = $state(0);
  played = $state(0);
  playing = $state(false);

  model = $derived(this.status?.model.state ?? null);
  speaking = $derived(this.status?.speaking ?? false);

  #timer: ReturnType<typeof setTimeout> | undefined;
  #context: AudioContext | null = null;
  #buffer: AudioBuffer | null = null;
  #source: AudioBufferSourceNode | null = null;
  #startedAt = 0;
  #frame = 0;

  /** `input`, a text given to the tool, takes the place of the one in the box. */
  load(input: string | undefined) {
    if (input) this.text = input;
    void this.#refresh();
  }

  async #refresh() {
    clearTimeout(this.#timer);
    const before = this.status;
    let status: Status;
    try {
      status = await native.call<Status>('status');
    } catch {
      toast(t('failures.other'));
      return;
    }
    this.status = status;
    if (status.model.failed) toast(t('failures.download'));
    if (status.failed) toast(t('failures.speak'));
    if (before?.model.state === 'downloading' && status.model.state === 'ready') toast(t('ready'));
    if (before?.speaking && !status.speaking && status.seconds !== null) await this.#fetchAudio();
    if (status.model.state === 'downloading' || status.speaking) {
      this.#timer = setTimeout(() => void this.#refresh(), POLL_MS);
    }
  }

  setLanguage(language: Language) {
    if (this.text === SAMPLES[this.language]) this.text = SAMPLES[language];
    this.language = language;
  }

  async paste() {
    try {
      const text = await navigator.clipboard.readText();
      if (text) this.text = text;
    } catch {
      toast(t('failures.other'));
    }
  }

  download() {
    native.call('download').then(
      () => void this.#refresh(),
      () => toast(t('failures.download')),
    );
  }

  generate() {
    if (!this.text.trim()) return;
    this.stop();
    native
      .call('speak', {
        text: this.text,
        language: this.language,
        voice: this.voice,
        speed: this.speed,
      })
      .then(
        () => {
          if (this.status) this.status = { ...this.status, speaking: true, percent: 0 };
          void this.#refresh();
        },
        () => toast(t('failures.speak')),
      );
  }

  cancel() {
    native.call('cancel').then(
      () => void this.#refresh(),
      () => toast(t('failures.other')),
    );
  }

  save() {
    native.call<string | null>('save', { format: this.format }).then(
      (file) => {
        if (file) toast(t('saved', { file }));
      },
      () => toast(t('failures.save')),
    );
  }

  async #fetchAudio() {
    try {
      const base64 = await native.call<string>('audio');
      this.#context ??= new AudioContext();
      this.#buffer = await this.#context.decodeAudioData(bytesOf(base64));
      this.duration = this.#buffer.duration;
      this.played = 0;
    } catch {
      toast(t('failures.speak'));
    }
  }

  togglePlay() {
    if (this.playing) {
      this.stop();
      return;
    }
    const context = this.#context;
    const buffer = this.#buffer;
    if (!context || !buffer) return;
    if (this.played >= this.duration) this.played = 0;
    const source = context.createBufferSource();
    source.buffer = buffer;
    source.connect(context.destination);
    source.onended = () => {
      if (this.#source !== source) return;
      this.#source = null;
      this.playing = false;
      cancelAnimationFrame(this.#frame);
      this.played = this.duration;
    };
    void context.resume();
    source.start(0, this.played);
    this.#source = source;
    this.#startedAt = context.currentTime - this.played;
    this.playing = true;
    const follow = () => {
      this.played = Math.min(this.duration, context.currentTime - this.#startedAt);
      this.#frame = requestAnimationFrame(follow);
    };
    follow();
  }

  /** Pauses where it is: the next play goes on from there. */
  stop() {
    const source = this.#source;
    if (!source) return;
    this.#source = null;
    cancelAnimationFrame(this.#frame);
    source.stop();
    this.playing = false;
  }
}

export const tts = new Tts();
