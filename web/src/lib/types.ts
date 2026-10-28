export interface Artist {
  id: number;
  name: string;
}

export interface Track {
  id: number;
  name: string;
  alias: string | null;
  artists: Artist[];
  albumId: number;
  album: string;
  cover: string;
  duration: number;
  fee: number;
}

export interface Collection {
  kind: 'playlist' | 'album';
  id: number;
  name: string;
  cover: string;
  owner: string;
  description: string;
  publishTime: number | null;
  trackCount: number;
  tracks: Track[];
}

export interface SongUrl {
  id: number;
  url: string;
  level: string;
  size: number;
  br: number;
  format: string;
  /** 试听片段的起止毫秒，非空表示不是完整歌曲 */
  trial: [number, number] | null;
}

export interface Lyric {
  lrc: string;
  tlrc: string;
}

export interface Account {
  userId: number;
  nickname: string;
  avatar: string;
  vipType: number;
}

export interface QrStart {
  key: string;
  url: string;
  svg: string;
}

export type QrState = 'waiting' | 'scanned' | 'expired' | 'success';

export type Resolved =
  | { type: 'song'; value: number }
  | { type: 'playlist'; value: number }
  | { type: 'album'; value: number }
  | { type: 'query'; value: string };

export type JobStatus = 'waiting' | 'running' | 'paused' | 'done' | 'failed' | 'cancelled';

export interface ItemState {
  id: number;
  name: string;
  artist: string;
  status: JobStatus;
  done: number;
  total: number;
  error: string | null;
  file: string | null;
}

export interface JobSummary {
  id: number;
  title: string;
  subtitle: string;
  cover: string;
  status: JobStatus;
  total: number;
  completed: number;
  failed: number;
  speed: number;
  progress: number;
  running: ItemState[];
  hasArtifact: boolean;
  dir: string;
  created: number;
}

export interface JobDetail {
  id: number;
  title: string;
  status: JobStatus;
  items: ItemState[];
  dir: string;
}

export interface DownloadOptions {
  level: string;
  template: string;
  writeMeta: boolean;
  writeCover: boolean;
  compressCover: boolean;
  writeArtist: boolean;
  writeAlbum: boolean;
  writeTrack: boolean;
  writeLyrics: boolean;
  lrcFile: boolean;
}

export interface DownloadItem {
  track: Track;
  index: number | null;
  total: number | null;
}

export interface NewJob {
  title: string;
  subtitle: string;
  cover: string;
  items: DownloadItem[];
  options: DownloadOptions;
  dir?: string;
}

export const LEVELS: { value: string; label: string; hint: string }[] = [
  { value: 'standard', label: '标准', hint: '128 kbps MP3' },
  { value: 'exhigh', label: '极高', hint: '320 kbps MP3' },
  { value: 'lossless', label: '无损', hint: 'FLAC，需要会员' },
  { value: 'hires', label: 'Hi-Res', hint: '高解析 FLAC，需要会员' },
  { value: 'sky', label: '沉浸环绕声', hint: '需要会员' },
  { value: 'jyeffect', label: '高清环绕声', hint: '需要会员' },
  { value: 'jymaster', label: '超清母带', hint: '需要会员' },
];
