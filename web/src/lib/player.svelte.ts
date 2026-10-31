import { api } from './transport';
import { settings } from './settings.svelte';
import { parseLyric, type LrcLine } from './lrc';
import { toast } from './toast.svelte';
import { artists, img } from './format';
import type { Track } from './types';
import { extractTint } from './tint';
import { recordPlay } from './recent.svelte';
import type { MatchCandidate } from './types';

export type RepeatMode = 'off' | 'all' | 'one';

class Player {
  queue = $state<Track[]>([]);
  index = $state(-1);
  playing = $state(false);
  loading = $state(false);
  /** 秒，约 10Hz 更新，用于进度条与时间显示 */
  time = $state(0);
  /** 秒，逐帧更新，仅供歌词同步使用 */
  frameTime = $state(0);
  duration = $state(0);
  buffered = $state(0);
  lyric = $state<LrcLine[]>([]);
  repeat = $state<RepeatMode>('all');
  shuffle = $state(false);
  expanded = $state(false);
  quality = $state('');
  /** 当前封面主色 "r g b" */
  tint = $state('');
  /** 当前只能播放试听片段 */
  trial = $state(false);
  /** 换源候选中 */
  matching = $state(false);
  /** 已完成恢复标记，供界面判断是否还要恢复播放位置 */
  restored = $state(false);

  current = $derived(this.index >= 0 ? (this.queue[this.index] ?? null) : null);

  #audio: HTMLAudioElement;
  #urlCache = new Map<string, { url: string; level: string; trial: boolean; from: string; exp: number }>();
  #token = 0;

  constructor() {
    const a = new Audio();
    a.preload = 'auto';
    a.volume = settings.volume;
    a.addEventListener('play', () => {
      this.playing = true;
      this.#startTicker();
      if (this.current) recordPlay(this.current);
    });
    a.addEventListener('pause', () => {
      this.playing = false;
      this.time = this.frameTime = a.currentTime;
      this.#syncPosition();
    });
    a.addEventListener('waiting', () => (this.loading = true));
    a.addEventListener('playing', () => (this.loading = false));
    a.addEventListener('canplay', () => (this.loading = false));
    a.addEventListener('durationchange', () => (this.duration = isFinite(a.duration) ? a.duration : 0));
    // 播放时由 rAF 更新进度，暂停或后台时退回 timeupdate
    a.addEventListener('timeupdate', () => {
      if (a.paused || document.hidden) this.time = this.frameTime = a.currentTime;
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
      if (this.#urlCache.delete(this.#cacheKey(this.current))) this.#load(true);
      else toast.error('播放失败', this.current.name);
    });
    this.#audio = a;
    this.#bindMediaSession();
  }

  #persist() {
    try {
      const t = this.current;
      localStorage.setItem(
        'shiyin.session',
        JSON.stringify({
          queue: this.queue,
          index: this.index,
          time: this.#audio.currentTime || this.time,
          repeat: this.repeat,
          shuffle: this.shuffle,
        }),
      );
      void t;
    } catch {
      /* 隐私模式等场景下配额不足，忽略 */
    }
  }

  /** 刷新后恢复上次的队列与进度（暂停态，不自动播放） */
  async restore() {
    type Saved = { queue: Track[]; index: number; time: number; repeat?: RepeatMode; shuffle?: boolean };
    let saved: Saved | null = null;
    try {
      saved = JSON.parse(localStorage.getItem('shiyin.session') ?? 'null');
    } catch {
      saved = null;
    }
    if (!saved?.queue?.length || saved.index == null || saved.index < 0 || saved.index >= saved.queue.length) {
      this.restored = true;
      return;
    }
    this.queue = saved.queue;
    this.index = saved.index;
    if (saved.repeat) this.repeat = saved.repeat;
    if (typeof saved.shuffle === 'boolean') this.shuffle = saved.shuffle;
    const t = this.current!;
    this.duration = t.duration / 1000;
    this.time = this.frameTime = Math.min(saved.time || 0, this.duration || 0);
    this.#updateMetadata();
    extractTint(t.cover).then((c) => (this.tint = c));
    api.lyric(t.id, t.source).then((l) => (this.lyric = parseLyric(l.lrc, l.tlrc))).catch(() => {});
    try {
      const u = await this.#resolveUrl(t);
      this.quality = u.level;
      this.trial = u.trial;
      this.matchedFrom = u.from !== t.source ? u.from : '';
      this.#srcAt = Date.now();
      this.#audio.src = u.url;
      this.#audio.addEventListener('loadedmetadata', () => {
        if (this.#audio.duration > 0 && this.time < this.#audio.duration) this.#audio.currentTime = this.time;
      }, { once: true });
    } catch {
      /* 恢复失败就保持暂停，用户点播放时会重新解析 */
    }
    this.restored = true;
  }

  #raf = 0;
  #lastTimeWrite = 0;
  #lastPersist = 0;
  #startTicker() {
    if (this.#raf) return;
    const tick = (now: number) => {
      if (this.#audio.paused) {
        this.#raf = 0;
        return;
      }
      if (!document.hidden) {
        const cur = this.#audio.currentTime;
        // 歌词需要逐帧；进度条与时间文字 10Hz 足够，避免整棵组件树每帧重算
        this.frameTime = cur;
        if (now - this.#lastTimeWrite >= 100) {
          this.#lastTimeWrite = now;
          this.time = cur;
        }
        if (now - this.#lastPersist >= 5000) {
          this.#lastPersist = now;
          this.#persist();
        }
      }
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

  /** 实际播放所用的音源（发生自动换源时与 current.source 不同） */
  matchedFrom = $state('');

  async #resolveUrl(t: Track): Promise<{ url: string; level: string; trial: boolean; from: string }> {
    const key = this.#cacheKey(t);
    const c = this.#urlCache.get(key);
    if (c && c.exp > Date.now()) return c;
    try {
      const r = await api.songUrl(t.id, settings.playLevel, t.source);
      if (r.trial && settings.autoFallback) {
        const alt = await this.#tryFallback(t);
        if (alt) return alt;
      }
      const v = { url: r.url, level: r.level, trial: !!r.trial, from: t.source, exp: Date.now() + 15 * 60_000 };
      this.#urlCache.set(key, v);
      return v;
    } catch (origErr) {
      if (settings.autoFallback) {
        const alt = await this.#tryFallback(t);
        if (alt) return alt;
      }
      throw origErr;
    }
  }

  async #tryFallback(t: Track) {
    try {
      const m = await api.matchPlayable(t.name, artists(t), t.source, settings.playLevel, t.duration);
      const v = { url: m.url.url, level: m.url.level, trial: false, from: m.track.source, exp: Date.now() + 15 * 60_000 };
      this.#urlCache.set(this.#cacheKey(t), v);
      return v;
    } catch {
      return null;
    }
  }

  async #load(keepTime = false) {
    const t = this.current;
    if (!t) return;
    const token = ++this.#token;
    const resume = this.#pendingSeek ?? (keepTime ? this.#audio.currentTime : 0);
    this.#pendingSeek = null;
    this.loading = true;
    if (!keepTime) {
      this.time = this.frameTime = 0;
      this.duration = t.duration / 1000;
      this.buffered = 0;
      this.lyric = [];
    }
    this.#updateMetadata();
    extractTint(t.cover).then((c) => {
      if (token === this.#token) this.tint = c;
    });
    try {
      const [u, l] = await Promise.all([
        this.#resolveUrl(t),
        keepTime ? Promise.resolve(null) : api.lyric(t.id, t.source).catch(() => null),
      ]);
      if (token !== this.#token) return;
      if (l) this.lyric = parseLyric(l.lrc, l.tlrc);
      this.#srcAt = Date.now();
      this.quality = u.level;
      this.trial = u.trial;
      this.matchedFrom = u.from !== t.source ? u.from : '';
      if (u.trial) toast.info(`「${t.name}」只能试听片段`, '登录会员账号后可播放完整歌曲');
      this.#audio.src = u.url;
      if (resume) {
        const a = this.#audio;
        a.addEventListener('loadedmetadata', () => (a.currentTime = resume), { once: true });
      }
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
  /** 当前 src 的获取时间，用于判断地址是否快要过期 */
  #srcAt = 0;
  #stale() {
    return !!this.#audio.src && Date.now() - this.#srcAt > 15 * 60_000;
  }

  #preloadNext() {
    this.#skipStreak = 0;
    const n = this.#nextIndex(false);
    if (n >= 0 && n !== this.index) this.#resolveUrl(this.queue[n]).catch(() => {});
  }

  /** 下一首可播放歌曲的位置，跳过无版权歌曲 */
  #nextIndex(manual: boolean, dir = 1): number {
    const len = this.queue.length;
    if (!len) return -1;
    if (this.repeat === 'one' && !manual) return this.index;
    const playable = (i: number) => this.queue[i].availability !== 'none';
    if (this.shuffle && len > 1) {
      const cands = this.queue.map((_, i) => i).filter((i) => i !== this.index && playable(i));
      return cands.length ? cands[Math.floor(Math.random() * cands.length)] : -1;
    }
    for (let step = 1; step <= len; step++) {
      let n = this.index + step * dir;
      if (n >= len || n < 0) {
        if (this.repeat === 'off' && !manual) return -1;
        n = ((n % len) + len) % len;
      }
      if (playable(n)) return n;
    }
    return -1;
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
    // 起点不可播放时顺延到下一首可播放的
    let i = start;
    for (let k = 0; k < tracks.length && tracks[i].availability === 'none'; k++) i = (i + 1) % tracks.length;
    if (tracks[i].availability === 'none') {
      toast.error('列表中的歌曲都无法播放', '可能是版权限制');
      return;
    }
    this.index = i;
    this.#load();
    this.#persist();
  }

  /** 播放单曲：已在队列中则跳转，否则插入到当前曲目之后 */
  play(t: Track) {
    if (t.availability === 'none' && !settings.autoFallback) {
      toast.error(`「${t.name}」暂无版权`, '无法播放');
      return;
    }
    const i = this.queue.findIndex((x) => x.id === t.id && x.source === t.source);
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
    this.#persist();
  }

  enqueue(tracks: Track[]) {
    const ids = new Set(this.queue.map((t) => `${t.source}:${t.id}`));
    const add = tracks.filter((t) => !ids.has(`${t.source}:${t.id}`));
    this.queue.push(...add);
    if (this.index < 0 && this.queue.length) {
      this.index = 0;
      this.#load();
    }
    return add.length;
  }

  playNext(t: Track) {
    const i = this.queue.findIndex((x) => x.id === t.id && x.source === t.source);
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

  /** 队列内移动（拖拽排序 / 上下移动） */
  move(from: number, to: number) {
    const len = this.queue.length;
    if (from === to || from < 0 || from >= len || to < 0 || to >= len) return;
    const next = this.queue.slice();
    const [item] = next.splice(from, 1);
    next.splice(to, 0, item);
    this.queue = next;
    if (this.index === from) this.index = to;
    else if (from < this.index && to >= this.index) this.index--;
    else if (from > this.index && to <= this.index) this.index++;
    this.#persist();
  }

  /** 手动换源：用其它平台的同名版本替换当前曲目并立即播放 */
  async switchSource(c: MatchCandidate) {
    const i = this.index;
    if (i < 0) return;
    const original = this.queue[i];
    this.#urlCache.set(this.#cacheKey(c.track), {
      url: c.url.url,
      level: c.url.level,
      trial: false,
      from: c.track.source,
      exp: Date.now() + 15 * 60_000,
    });
    const resume = this.#audio.currentTime;
    const wasPlaying = !this.#audio.paused;
    this.queue = this.queue.map((t, k) => (k === i ? c.track : t));
    this.matchedFrom = c.track.source !== original.source ? original.source : '';
    await this.#load(true);
    if (resume > 0) this.seek(resume);
    if (!wasPlaying) this.#audio.pause();
    this.#persist();
  }

  /** 重新解析当前曲目（音质切换、换源失败重试） */
  async retryCurrent() {
    const t = this.current;
    if (!t) return;
    this.#urlCache.delete(this.#cacheKey(t));
    await this.#load(true);
  }

  stop() {
    this.#token++;
    this.#audio.pause();
    this.#audio.removeAttribute('src');
    this.#audio.load();
    this.index = -1;
    this.time = this.frameTime = 0;
    this.duration = 0;
    this.lyric = [];
    this.expanded = false;
    try {
      localStorage.removeItem('shiyin.session');
    } catch {
      /* 忽略 */
    }
  }

  toggle() {
    if (!this.current) return;
    if (!this.#audio.src) {
      // 恢复会话时地址解析失败：从记忆的位置继续
      if (this.time > 0) this.#pendingSeek = this.time;
      return void this.#load(true);
    }
    if (this.#audio.paused) {
      // 暂停太久后地址可能已过期，先刷新再从原位置继续
      if (this.#stale()) return void this.#refresh();
      this.#audio.play();
    } else this.#audio.pause();
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
    const n = this.#nextIndex(true, -1);
    if (n < 0) return;
    this.index = n;
    this.#load();
  }

  seek(sec: number) {
    if (!this.#audio.src) return;
    if (this.#stale()) {
      this.#pendingSeek = sec;
      return void this.#refresh();
    }
    this.#audio.currentTime = Math.max(0, Math.min(sec, this.duration || sec));
    this.time = this.frameTime = this.#audio.currentTime;
  }

  cycleRepeat() {
    this.repeat = this.repeat === 'all' ? 'one' : this.repeat === 'one' ? 'off' : 'all';
  }

  #pendingSeek: number | null = null;
  #refresh() {
    const t = this.current;
    if (t) this.#urlCache.delete(this.#cacheKey(t));
    this.#load(true);
  }

  #cacheKey(t: Track) {
    return `${t.source}:${t.id}:${settings.playLevel}`;
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
