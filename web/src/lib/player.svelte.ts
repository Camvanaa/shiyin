import { api } from './transport';
import { settings } from './settings.svelte';
import { parseLyric, type LrcLine } from './lrc';
import { toast } from './toast.svelte';
import { artists, img } from './format';
import type { Track } from './types';

export type RepeatMode = 'off' | 'all' | 'one';

class Player {
  queue = $state<Track[]>([]);
  index = $state(-1);
  playing = $state(false);
  loading = $state(false);
  /** 秒 */
  time = $state(0);
  duration = $state(0);
  buffered = $state(0);
  lyric = $state<LrcLine[]>([]);
  repeat = $state<RepeatMode>('all');
  shuffle = $state(false);
  expanded = $state(false);
  quality = $state('');

  current = $derived(this.index >= 0 ? (this.queue[this.index] ?? null) : null);

  #audio: HTMLAudioElement;
  #urlCache = new Map<string, { url: string; level: string; exp: number }>();
  #token = 0;

  constructor() {
    const a = new Audio();
    a.preload = 'auto';
    a.volume = settings.volume;
    a.addEventListener('play', () => {
      this.playing = true;
      this.#startTicker();
    });
    a.addEventListener('pause', () => {
      this.playing = false;
      this.time = a.currentTime;
      this.#syncPosition();
    });
    a.addEventListener('waiting', () => (this.loading = true));
    a.addEventListener('playing', () => (this.loading = false));
    a.addEventListener('canplay', () => (this.loading = false));
    a.addEventListener('durationchange', () => (this.duration = isFinite(a.duration) ? a.duration : 0));
    // 播放时由 rAF 更新进度，暂停或后台时退回 timeupdate
    a.addEventListener('timeupdate', () => {
      if (a.paused || document.hidden) this.time = a.currentTime;
    });
    a.addEventListener('seeked', () => this.#syncPosition());
    a.addEventListener('ratechange', () => this.#syncPosition());
    a.addEventListener('progress', () => {
      if (a.buffered.length) this.buffered = a.buffered.end(a.buffered.length - 1);
    });
    a.addEventListener('ended', () => this.#onEnded());
    a.addEventListener('error', () => {
      if (!a.src || !this.current) return;
      this.loading = false;
      // 地址可能过期，丢弃缓存后重试一次
      const key = `${this.current.id}:${settings.playLevel}`;
      if (this.#urlCache.delete(key)) this.#load(true);
      else toast.error('播放失败', this.current.name);
    });
    this.#audio = a;
    this.#bindMediaSession();
  }

  #raf = 0;
  #startTicker() {
    if (this.#raf) return;
    const tick = () => {
      if (this.#audio.paused) {
        this.#raf = 0;
        return;
      }
      if (!document.hidden) this.time = this.#audio.currentTime;
      this.#raf = requestAnimationFrame(tick);
    };
    this.#raf = requestAnimationFrame(tick);
    this.#syncPosition();
  }

  #syncPosition() {
    const ms = navigator.mediaSession;
    const a = this.#audio;
    if (!ms?.setPositionState || !isFinite(a.duration) || a.duration <= 0) return;
    try {
      ms.setPositionState({ duration: a.duration, position: Math.min(a.currentTime, a.duration), playbackRate: a.playbackRate });
    } catch {
      /* 某些 WebView 不支持 */
    }
  }

  get volume() {
    return settings.volume;
  }
  set volume(v: number) {
    settings.volume = Math.min(1, Math.max(0, v));
    this.#audio.volume = settings.volume;
  }

  async #resolveUrl(t: Track): Promise<{ url: string; level: string }> {
    const key = `${t.id}:${settings.playLevel}`;
    const c = this.#urlCache.get(key);
    if (c && c.exp > Date.now()) return c;
    const r = await api.songUrl(t.id, settings.playLevel);
    // 网易云的地址一般 20 分钟后失效，保守缓存 15 分钟
    const v = { url: r.url, level: r.level, exp: Date.now() + 15 * 60_000 };
    this.#urlCache.set(key, v);
    return v;
  }

  async #load(keepTime = false) {
    const t = this.current;
    if (!t) return;
    const token = ++this.#token;
    const resume = keepTime ? this.#audio.currentTime : 0;
    this.loading = true;
    if (!keepTime) {
      this.time = 0;
      this.duration = t.duration / 1000;
      this.buffered = 0;
      this.lyric = [];
    }
    this.#updateMetadata();
    try {
      const [u, l] = await Promise.all([
        this.#resolveUrl(t),
        keepTime ? Promise.resolve(null) : api.lyric(t.id).catch(() => null),
      ]);
      if (token !== this.#token) return;
      if (l) this.lyric = parseLyric(l.lrc, l.tlrc);
      this.quality = u.level;
      this.#audio.src = u.url;
      if (resume) this.#audio.currentTime = resume;
      await this.#audio.play().catch(() => {});
      this.#preloadNext();
    } catch (e) {
      if (token !== this.#token) return;
      this.loading = false;
      toast.error(`无法播放「${t.name}」`, (e as Error).message);
      // 自动跳过不可播放的歌曲，但避免整队列都失败时无限循环
      if (this.queue.length > 1 && this.#skipStreak++ < 3) this.next();
    }
  }
  #skipStreak = 0;

  #preloadNext() {
    this.#skipStreak = 0;
    const n = this.#nextIndex(false);
    if (n >= 0 && n !== this.index) this.#resolveUrl(this.queue[n]).catch(() => {});
  }

  #nextIndex(manual: boolean): number {
    const len = this.queue.length;
    if (!len) return -1;
    if (this.repeat === 'one' && !manual) return this.index;
    if (this.shuffle && len > 1) {
      let r = this.index;
      while (r === this.index) r = Math.floor(Math.random() * len);
      return r;
    }
    const n = this.index + 1;
    if (n < len) return n;
    return this.repeat === 'off' && !manual ? -1 : 0;
  }

  #onEnded() {
    const n = this.#nextIndex(false);
    if (n < 0) {
      this.playing = false;
      return;
    }
    if (n === this.index) {
      this.#audio.currentTime = 0;
      this.#audio.play();
      return;
    }
    this.index = n;
    this.#load();
  }

  /** 替换队列并播放 */
  playList(tracks: Track[], start = 0) {
    if (!tracks.length) return;
    this.queue = tracks.slice();
    this.index = start;
    this.#load();
  }

  /** 播放单曲：已在队列中则跳转，否则插入到当前曲目之后 */
  play(t: Track) {
    const i = this.queue.findIndex((x) => x.id === t.id);
    if (i >= 0) {
      if (i === this.index) return this.toggle();
      this.index = i;
    } else {
      this.queue.splice(this.index + 1, 0, t);
      this.index = this.index + 1;
    }
    this.#load();
  }

  playAt(i: number) {
    if (i < 0 || i >= this.queue.length) return;
    this.index = i;
    this.#load();
  }

  enqueue(tracks: Track[]) {
    const ids = new Set(this.queue.map((t) => t.id));
    const add = tracks.filter((t) => !ids.has(t.id));
    this.queue.push(...add);
    if (this.index < 0 && this.queue.length) {
      this.index = 0;
      this.#load();
    }
    return add.length;
  }

  playNext(t: Track) {
    const i = this.queue.findIndex((x) => x.id === t.id);
    if (i === this.index && i >= 0) return;
    if (i >= 0) {
      this.queue.splice(i, 1);
      if (i < this.index) this.index--;
    }
    this.queue.splice(this.index + 1, 0, t);
    if (this.index < 0) {
      this.index = 0;
      this.#load();
    }
  }

  remove(i: number) {
    if (i === this.index) {
      this.queue.splice(i, 1);
      if (!this.queue.length) return this.stop();
      this.index = Math.min(i, this.queue.length - 1);
      this.#load();
    } else {
      this.queue.splice(i, 1);
      if (i < this.index) this.index--;
    }
  }

  clear() {
    this.queue = [];
    this.stop();
  }

  stop() {
    this.#token++;
    this.#audio.pause();
    this.#audio.removeAttribute('src');
    this.#audio.load();
    this.index = -1;
    this.time = 0;
    this.duration = 0;
    this.lyric = [];
    this.expanded = false;
  }

  toggle() {
    if (!this.current) return;
    if (!this.#audio.src) return void this.#load();
    if (this.#audio.paused) this.#audio.play();
    else this.#audio.pause();
  }

  next() {
    const n = this.#nextIndex(true);
    if (n >= 0) {
      this.index = n;
      this.#load();
    }
  }

  prev() {
    if (this.#audio.currentTime > 3) {
      this.seek(0);
      return;
    }
    if (!this.queue.length) return;
    this.index = (this.index - 1 + this.queue.length) % this.queue.length;
    this.#load();
  }

  seek(sec: number) {
    if (!this.#audio.src) return;
    this.#audio.currentTime = Math.max(0, Math.min(sec, this.duration || sec));
    this.time = this.#audio.currentTime;
  }

  cycleRepeat() {
    this.repeat = this.repeat === 'all' ? 'one' : this.repeat === 'one' ? 'off' : 'all';
  }

  /** 音质设置变化后重新加载当前歌曲 */
  reloadQuality() {
    if (this.current && this.#audio.src) this.#load(true);
  }

  #bindMediaSession() {
    const ms = navigator.mediaSession;
    if (!ms) return;
    ms.setActionHandler('play', () => this.#audio.play());
    ms.setActionHandler('pause', () => this.#audio.pause());
    ms.setActionHandler('previoustrack', () => this.prev());
    ms.setActionHandler('nexttrack', () => this.next());
    const safe = (a: MediaSessionAction, h: MediaSessionActionHandler) => {
      try {
        ms.setActionHandler(a, h);
      } catch {
        /* 不支持的动作 */
      }
    };
    safe('seekto', (d) => d.seekTime != null && this.seek(d.seekTime));
    safe('seekbackward', (d) => this.seek(this.#audio.currentTime - (d.seekOffset ?? 10)));
    safe('seekforward', (d) => this.seek(this.#audio.currentTime + (d.seekOffset ?? 10)));
  }

  #updateMetadata() {
    const t = this.current;
    if (!t || !navigator.mediaSession || typeof MediaMetadata === 'undefined') return;
    navigator.mediaSession.metadata = new MediaMetadata({
      title: t.name,
      artist: artists(t),
      album: t.album,
      artwork: t.cover ? [{ src: img(t.cover, 256), sizes: '512x512', type: 'image/jpeg' }] : [],
    });
    document.title = `${t.name} · ${artists(t)}`;
  }
}

export const player = new Player();
