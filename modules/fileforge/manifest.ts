import { defineModule } from '@arthex/sdk';
import { AUDIO_FORMATS, DOCUMENT_FORMATS, IMAGE_FORMATS, VIDEO_FORMATS, formatLabel } from './formats';
import { ICON, ID, TINT } from './identity';
import { openOnRightClick } from './opening';

const IMAGES = ['.png', '.jpg', '.jpeg', '.webp', '.avif', '.gif', '.bmp', '.tif', '.tiff', '.ico', '.heic'];
const VIDEOS = ['.mp4', '.mov', '.mkv', '.avi', '.webm', '.m4v', '.wmv', '.flv', '.mpg', '.mpeg'];
const AUDIOS = ['.mp3', '.wav', '.ogg', '.flac', '.m4a', '.aac', '.opus', '.wma'];
const DOCUMENTS = ['.pdf', '.docx', '.doc', '.odt', '.rtf', '.txt', '.html', '.htm'];
const COMPRESSED_DOCUMENTS = ['.pdf', '.docx', '.pptx', '.xlsx', '.odt'];

const choices = (formats: readonly string[]) =>
  formats.map((format) => ({ id: format, label: formatLabel(format) }));

openOnRightClick();

export default defineModule({
  id: ID,
  version: '1.1.0',
  api: 9,
  uses: ['ffmpeg'],
  icon: ICON,
  tint: TINT,
  order: 66,
  tags: ['files', 'media'],
  locales: import.meta.glob('./locales/*.json', { eager: true, import: 'default' }),
  background: true,
  page: () => import('./ForgePage.svelte'),
  contextMenu: [
    {
      id: 'compress',
      targets: ['file'],
      extensions: [...IMAGES, ...VIDEOS, ...AUDIOS, ...COMPRESSED_DOCUMENTS],
    },
    { id: 'convert-image', targets: ['file'], extensions: IMAGES, submenu: choices(IMAGE_FORMATS) },
    {
      id: 'convert-video',
      targets: ['file'],
      extensions: VIDEOS,
      submenu: choices([...VIDEO_FORMATS, ...AUDIO_FORMATS]),
    },
    { id: 'convert-audio', targets: ['file'], extensions: AUDIOS, submenu: choices(AUDIO_FORMATS) },
    { id: 'convert-document', targets: ['file'], extensions: DOCUMENTS, submenu: choices(DOCUMENT_FORMATS) },
  ],
});
