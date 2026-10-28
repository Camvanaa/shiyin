import type { DownloadOptions } from './types';

export interface Settings extends DownloadOptions {
  playLevel: string;
  searchLimit: number;
  concurrency: number;
  theme: 'system' | 'light' | 'dark';
  /** 桌面版下载目录，空字符串表示系统“下载”文件夹 */
  dir: string;
  /** 合集下载时是否在目录下建立子文件夹 */
  subfolder: boolean;
  volume: number;
}

const KEY = 'shiyin.settings';

const defaults: Settings = {
  level: 'exhigh',
  template: '${title} - ${artist}',
  writeMeta: true,
  writeCover: true,
  compressCover: true,
  writeArtist: true,
  writeAlbum: true,
  writeTrack: true,
  writeLyrics: true,
  lrcFile: false,
  playLevel: 'exhigh',
  searchLimit: 30,
  concurrency: 3,
  theme: 'system',
  dir: '',
  subfolder: true,
  volume: 0.8,
};

function load(): Settings {
  try {
    return { ...defaults, ...JSON.parse(localStorage.getItem(KEY) ?? '{}') };
  } catch {
    return { ...defaults };
  }
}

export const settings = $state<Settings>(load());

let timer: ReturnType<typeof setTimeout> | undefined;
$effect.root(() => {
  $effect(() => {
    const snapshot = JSON.stringify(settings);
    clearTimeout(timer);
    timer = setTimeout(() => localStorage.setItem(KEY, snapshot), 200);
  });
  $effect(() => {
    const t = settings.theme;
    const root = document.documentElement;
    if (t === 'system') root.removeAttribute('data-theme');
    else root.dataset.theme = t;
  });
});

export function downloadOptions(): DownloadOptions {
  const { level, template, writeMeta, writeCover, compressCover, writeArtist, writeAlbum, writeTrack, writeLyrics, lrcFile } = settings;
  return { level, template, writeMeta, writeCover, compressCover, writeArtist, writeAlbum, writeTrack, writeLyrics, lrcFile };
}
