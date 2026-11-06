import { hideOverlay, moduleNative, toast } from '@arthex/sdk';
import { t } from './i18n';
import { ID } from './identity';
import {
  AUDIO_FORMATS,
  defaultBitrate,
  defaultHeight,
  isLossless,
  toLink,
  type AudioQuality,
  type VideoQuality,
} from './links';

const READ_AFTER_MS = 350;
const native = moduleNative(ID);

export type Failure = 'unsupported' | 'protected' | 'unavailable' | 'network' | 'other';

export interface Info {
  title: string;
  author: string;
  site: string;
  duration: number | null;
  audioOnly: boolean;
  videos: VideoQuality[];
  audios: AudioQuality[];
  videoFormats: string[];
}

export interface Status {
  running: boolean;
  percent: number;
  speed: number | null;
  eta: number | null;
  processing: boolean;
  file: string | null;
  failure: Failure | null;
  cancelled: boolean;
}

interface Analysed {
  token: number;
  info?: Info;
  thumbnail?: string | null;
  failure?: Failure;
}

const IDLE: Status = {
  running: false,
  percent: 0,
  speed: null,
  eta: null,
  processing: false,
  file: null,
  failure: null,
  cancelled: false,
};

/** Lives with the module, not the tool: a download goes on when the tool is closed. */
class Downloader {
  link = $state('');
  phase = $state<'empty' | 'loading' | 'ready'>('empty');
  linkFailure = $state<Failure | null>(null);
  info = $state<Info | null>(null);
  thumbnail = $state<string | null>(null);
  audio = $state(false);
  format = $state('mp4');
  quality = $state(0);
  folder = $state('');
  status = $state<Status>(IDLE);
  /** Choosing something else puts "Download" back in place of the finished file. */
  resultSeen = $state(false);

  busy = $derived(this.status.running);
  finished = $derived(!this.status.running && !this.resultSeen ? this.status.file : null);
  failure = $derived(this.linkFailure ?? (this.resultSeen ? null : this.status.failure));
  formats = $derived<readonly string[]>(this.audio ? AUDIO_FORMATS : (this.info?.videoFormats ?? []));

  #listening = false;
  #timer: ReturnType<typeof setTimeout> | undefined;
  /** The analysis whose answer is awaited, known once the program has answered the call. */
  #token = 0;
  #calling = 0;
  #early: Analysed | null = null;
  #readLink: string | null = null;

  load() {
    if (!this.#listening) {
      this.#listening = true;
      native.on('progress', (status) => (this.status = status as Status));
      native.on('analysed', (answer) => this.#onAnalysed(answer as Analysed));
    }
    native.call<{ status: Status; folder: string }>('state').then(
      ({ status, folder }) => {
        this.status = status;
        this.folder = folder;
      },
      () => toast(t('failedAction')),
    );
  }

  /** What a search result passed on. A download under way keeps the tool on what it downloads. */
  open(input: string | undefined) {
    if (input && !this.busy && input !== this.#readLink) {
      this.link = input;
      this.#read(input);
    }
  }

  setLink(text: string) {
    this.link = text;
    clearTimeout(this.#timer);
    const link = toLink(text);
    if (link && link === this.#readLink) return;
    this.#forget();
    if (!text.trim()) return;
    if (link) this.phase = 'loading';
    else this.linkFailure = 'unsupported';
    if (link) this.#timer = setTimeout(() => this.#read(link), READ_AFTER_MS);
  }

  #forget() {
    this.#token = 0;
    this.#early = null;
    this.#readLink = null;
    this.info = null;
    this.thumbnail = null;
    this.phase = 'empty';
    this.linkFailure = null;
    this.resultSeen = true;
  }

  #read(link: string) {
    clearTimeout(this.#timer);
    this.#forget();
    this.#readLink = link;
    this.phase = 'loading';
    this.#calling += 1;
    native.call<number>('analyse', { url: link }).then(
      (token) => {
        this.#calling -= 1;
        if (this.#readLink !== link) return;
        this.#token = token;
        if (this.#early?.token === token) this.#onAnalysed(this.#early);
      },
      () => {
        this.#calling -= 1;
        if (this.#readLink === link) this.#onAnalysed({ token: 0, failure: 'other' });
      },
    );
  }

  /** It can come before the call that started it has answered. */
  #onAnalysed(answer: Analysed) {
    if (this.#calling > 0) {
      this.#early = answer;
      return;
    }
    if (answer.token !== this.#token) return;
    this.#early = null;
    if (!answer.info) {
      this.phase = 'empty';
      this.#readLink = null;
      // That it cannot be read says the most about a link it knows nothing else of.
      this.linkFailure = answer.failure && answer.failure !== 'other' ? answer.failure : 'unsupported';
      return;
    }
    this.info = answer.info;
    this.thumbnail = answer.thumbnail ?? null;
    this.setAudio(answer.info.audioOnly);
    this.phase = 'ready';
  }

  setAudio(audio: boolean) {
    this.audio = audio;
    this.format = this.formats[0] ?? '';
    this.quality = audio ? defaultBitrate(this.info?.audios ?? []) : defaultHeight(this.info?.videos ?? []);
    this.resultSeen = true;
  }

  setFormat(format: string) {
    this.format = format;
    this.resultSeen = true;
  }

  setQuality(quality: number) {
    this.quality = quality;
    this.resultSeen = true;
  }

  /** The dialog would open behind the overlay. */
  pickFolder() {
    native.callHidden<string | null>('pickFolder').then(
      (folder) => {
        if (folder) this.folder = folder;
      },
      () => toast(t('failedAction')),
    );
  }

  start() {
    const url = this.#readLink;
    if (!url) return;
    this.resultSeen = false;
    this.status = { ...IDLE, running: true };
    native.call('start', { url, audio: this.audio, format: this.format, quality: this.quality }).catch(() => {
      this.status = { ...IDLE, failure: 'other' };
    });
  }

  cancel() {
    void native.call('cancel').catch(() => toast(t('failedAction')));
  }

  #openOutside(method: string) {
    void native.call(method).then(hideOverlay, () => toast(t('failedAction')));
  }

  openFile = () => this.#openOutside('openFile');
  showFile = () => this.#openOutside('showFile');
  hasQuality = $derived(!isLossless(this.format));
}

export const downloader = new Downloader();
