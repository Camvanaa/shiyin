// 传输层：桌面版走 Tauri invoke，Web 版走 HTTP。界面代码只使用这里导出的 api。
import type {
  Account, Captcha, Collection, HomeFeed, JobDetail, JobSummary, LibraryData, LibPlaylist, Lyric, MatchCandidate, NewJob, QrStart, QrState, Resolved,
  SongUrl, SourceInfo, SourceOption, Track, Work,
} from './types';

export const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

export interface Api {
  home(source?: string): Promise<HomeFeed>;
  sources(): Promise<SourceInfo[]>;
  resolve(q: string): Promise<Resolved>;
  search(q: string, limit: number, offset?: number, source?: string): Promise<Track[]>;
  /** 聚合搜索并按作品合并（同一首歌的不同平台版本归并为一） */
  searchMerged(q: string, limit: number, offset?: number): Promise<Work[]>;
  /** 解析器：给定候选版本（"source:id"，按推荐度排序），返回第一个可播版本 */
  resolveWork(candidates: string[], level: string): Promise<{ track: Track; url: SongUrl }>;
  song(id: string, source?: string): Promise<Track>;
  songUrl(id: string, level: string, source?: string): Promise<SongUrl>;
  lyric(id: string, source?: string): Promise<Lyric>;
  playlist(id: string, source?: string): Promise<Collection>;
  album(id: string, source?: string): Promise<Collection>;
  toplist(id: string, source?: string): Promise<Collection>;
  matchPlayable(name: string, artist: string, skip: string, level: string, duration?: number): Promise<{ track: Track; url: SongUrl }>;
  matchCandidates(name: string, artist: string, skip: string, level: string, duration?: number): Promise<MatchCandidate[]>;
  setSourceCookie(source: string, cookie: string | null): Promise<{ hasCookie: boolean }>;
  setSourceOption(source: string, key: string, value: string): Promise<{ options: SourceOption[] }>;
  /** 桌面版：应用内网页登录；Web 版不可用 */
  sourceWebLogin?(source: string): Promise<{ hasCookie: boolean }>;
  webLoginCancel?(): Promise<void>;
  sourceSmsCaptcha(source: string): Promise<Captcha>;
  sourceSmsSend(source: string, mobile: string, captchaToken: string, captchaCode: string): Promise<void>;
  sourceSmsLogin(source: string, mobile: string, smsCode: string): Promise<{ hasCookie: boolean }>;
  sourceQrStart(source: string): Promise<QrStart>;
  sourceQrCheck(source: string, key: string): Promise<{ state: QrState; hasCookie: boolean }>;
  account(): Promise<{ account: Account | null; hasCredential: boolean }>;
  setCookie(musicU: string): Promise<{ account: Account }>;
  logout(): Promise<void>;
  qrStart(): Promise<QrStart>;
  qrCheck(key: string): Promise<{ state: QrState; account: Account | null }>;
  addJob(job: NewJob): Promise<{ id: number }>;
  jobs(): Promise<JobSummary[]>;
  job(id: number): Promise<JobDetail>;
  jobAction(id: number, action: 'pause' | 'resume' | 'cancel' | 'retry'): Promise<void>;
  removeJob(id: number): Promise<void>;
  clearJobs(): Promise<void>;
  setConcurrency(n: number): Promise<void>;
  /** 订阅任务变化，返回取消函数 */
  onJobs(cb: (jobs: JobSummary[]) => void): () => void;
  /** Web 版：任务产物的下载地址；桌面版：在文件管理器中显示 */
  reveal(job: JobSummary): Promise<void>;
  /** 桌面版：选择下载目录 */
  pickDir?(): Promise<string | null>;
  defaultDir?(): Promise<string>;
  /** 资料库 */
  library(): Promise<LibraryData>;
  toggleLike(track: Track, alts?: string[]): Promise<{ liked: boolean }>;
  createPlaylist(name: string): Promise<LibPlaylist>;
  playlistAdd(id: string, items: { track: Track; alts?: string[] }[]): Promise<{ added: number }>;
  playlistUpdate(id: string, patch: { name?: string; remove?: string[] }): Promise<{ ok: boolean; removed: number; playlist: LibPlaylist | null }>;
  playlistDelete(id: string): Promise<{ ok: boolean }>;
  recordPlay(track: Track, alts?: string[]): Promise<void>;
  clearPlays(): Promise<void>;
}

// ---------- HTTP ----------

const TOKEN_KEY = 'shiyin.token';
export const token = {
  get: () => localStorage.getItem(TOKEN_KEY) ?? '',
  set: (v: string) => localStorage.setItem(TOKEN_KEY, v),
};

export class AuthError extends Error {}

async function http<T>(method: string, path: string, body?: unknown, signal?: AbortSignal): Promise<T> {
  const headers: Record<string, string> = {};
  const t = token.get();
  if (t) headers.Authorization = `Bearer ${t}`;
  if (body !== undefined) headers['Content-Type'] = 'application/json';
  const res = await fetch(`/api${path}`, {
    method,
    headers,
    body: body === undefined ? undefined : JSON.stringify(body),
    signal,
  });
  const data = await res.json().catch(() => ({}));
  if (res.status === 401) throw new AuthError(data.error ?? '需要访问口令');
  if (!res.ok) throw new Error(data.error ?? `请求失败（${res.status}）`);
  return data as T;
}

const enc = encodeURIComponent;

const httpApi: Api = {
  home: (source = '') => http('GET', `/home?source=${enc(source)}`),
  sources: () => http('GET', '/sources'),
  resolve: (q) => http('GET', `/resolve?q=${enc(q)}`),
  search: (q, limit, offset = 0, source = '') => http('GET', `/search?q=${enc(q)}&limit=${limit}&offset=${offset}&source=${enc(source)}`),
  searchMerged: (q, limit, offset = 0) => http('GET', `/search/merged?q=${enc(q)}&limit=${limit}&offset=${offset}`),
  resolveWork: (candidates, level) => http('POST', '/work/resolve', { candidates, level }),
  song: (id, source = '') => http('GET', `/song/${enc(id)}?source=${enc(source)}`),
  songUrl: (id, level, source = '') => http('GET', `/url/${enc(id)}?level=${enc(level)}&source=${enc(source)}`),
  lyric: (id, source = '') => http('GET', `/lyric/${enc(id)}?source=${enc(source)}`),
  playlist: (id, source = '') => http('GET', `/playlist/${enc(id)}?source=${enc(source)}`),
  album: (id, source = '') => http('GET', `/album/${enc(id)}?source=${enc(source)}`),
  toplist: (id, source = '') => http('GET', `/toplist/${enc(id)}?source=${enc(source)}`),
  matchPlayable: (name, artist, skip, level, duration = 0) => http('GET', `/match?name=${enc(name)}&artist=${enc(artist)}&skip=${enc(skip)}&level=${enc(level)}&duration=${duration}`),
  matchCandidates: (name, artist, skip, level, duration = 0) => http('GET', `/match/all?name=${enc(name)}&artist=${enc(artist)}&skip=${enc(skip)}&level=${enc(level)}&duration=${duration}`),
  setSourceCookie: (source, cookie) => cookie ? http('POST', `/sources/${enc(source)}/cookie`, { cookie }) : http('DELETE', `/sources/${enc(source)}/cookie`),
  setSourceOption: (source, key, value) => http('POST', `/sources/${enc(source)}/options`, { key, value }),
  sourceSmsCaptcha: (source) => http('GET', `/sources/${enc(source)}/sms/captcha`),
  sourceSmsSend: (source, mobile, captchaToken, captchaCode) => http('POST', `/sources/${enc(source)}/sms/send`, { mobile, captchaToken, captchaCode }),
  sourceSmsLogin: (source, mobile, smsCode) => http('POST', `/sources/${enc(source)}/sms/login`, { mobile, smsCode }),
  sourceQrStart: (source) => http('POST', `/sources/${enc(source)}/qr`),
  sourceQrCheck: (source, key) => http('GET', `/sources/${enc(source)}/qr/${enc(key)}`),
  account: () => http('GET', '/account'),
  setCookie: (musicU) => http('POST', '/account/cookie', { musicU }),
  logout: () => http('DELETE', '/account'),
  qrStart: () => http('POST', '/qr'),
  qrCheck: (key) => http('GET', `/qr/${enc(key)}`),
  addJob: (job) => http('POST', '/jobs', job),
  jobs: () => http('GET', '/jobs'),
  job: (id) => http('GET', `/jobs/${id}`),
  jobAction: (id, action) => http('POST', `/jobs/${id}/${action}`),
  removeJob: (id) => http('DELETE', `/jobs/${id}`),
  clearJobs: () => http('DELETE', '/jobs'),
  setConcurrency: (value) => http('POST', '/jobs/concurrency', { value }),
  onJobs(cb) {
    let es: EventSource | null = null;
    let closed = false;
    let retry = 1000;
    const open = () => {
      if (closed) return;
      const t = token.get();
      es = new EventSource(`/api/jobs/events${t ? `?token=${enc(t)}` : ''}`);
      es.onmessage = (e) => {
        retry = 1000;
        cb(JSON.parse(e.data));
      };
      es.onerror = () => {
        es?.close();
        setTimeout(open, retry);
        retry = Math.min(retry * 2, 15000);
      };
    };
    open();
    return () => {
      closed = true;
      es?.close();
    };
  },
  async reveal(job) {
    const t = token.get();
    const a = document.createElement('a');
    a.href = `/api/jobs/${job.id}/file${t ? `?token=${enc(t)}` : ''}`;
    a.download = '';
    document.body.appendChild(a);
    a.click();
    a.remove();
  },
  library: () => http('GET', '/library'),
  toggleLike: (track, alts = []) => http('POST', '/library/likes', { track, alts }),
  createPlaylist: (name) => http('POST', '/library/playlists', { name }),
  playlistAdd: (id, items) => http('POST', `/library/playlists/${enc(id)}/tracks`, { items }),
  playlistUpdate: (id, patch) => http('PATCH', `/library/playlists/${enc(id)}`, patch),
  playlistDelete: (id) => http('DELETE', `/library/playlists/${enc(id)}`),
  recordPlay: (track, alts = []) => http('POST', '/library/plays', { track, alts }),
  clearPlays: () => http('DELETE', '/library/plays'),
};

// ---------- Tauri ----------

async function tauriApi(): Promise<Api> {
  const { invoke } = await import('@tauri-apps/api/core');
  const { listen } = await import('@tauri-apps/api/event');
  const call = <T>(cmd: string, args?: Record<string, unknown>) =>
    invoke<T>(cmd, args).catch((e) => {
      throw new Error(typeof e === 'string' ? e : (e?.message ?? String(e)));
    });
  return {
    home: (source = '') => call('home', { source }),
    sources: () => call('sources'),
    resolve: (q) => call('resolve', { q }),
    search: (q, limit, offset = 0, source = '') => call('search', { q, limit, offset, source }),
    searchMerged: (q, limit, offset = 0) => call('search_merged', { q, limit, offset }),
    resolveWork: (candidates, level) => call('resolve_work', { candidates, level }),
    song: (id, source = '') => call('song', { id, source }),
    songUrl: (id, level, source = '') => call('song_url', { id, level, source }),
    lyric: (id, source = '') => call('lyric', { id, source }),
    playlist: (id, source = '') => call('playlist', { id, source }),
    album: (id, source = '') => call('album', { id, source }),
    toplist: (id, source = '') => call('toplist', { id, source }),
    matchPlayable: (name, artist, skip, level, duration = 0) => call('match_playable', { name, artist, skip, level, duration }),
    matchCandidates: (name, artist, skip, level, duration = 0) => call('match_candidates', { name, artist, skip, level, duration }),
    setSourceCookie: (source, cookie) => call('set_source_cookie', { source, cookie }),
    setSourceOption: (source, key, value) => call('set_source_option', { source, key, value }),
    sourceWebLogin: (source) => call('source_web_login', { source }),
    webLoginCancel: () => call('webview_login_cancel'),
    sourceSmsCaptcha: (source) => call('source_sms_captcha', { source }),
    sourceSmsSend: (source, mobile, captchaToken, captchaCode) => call('source_sms_send', { source, mobile, captchaToken, captchaCode }),
    sourceSmsLogin: (source, mobile, smsCode) => call('source_sms_login', { source, mobile, smsCode }),
    sourceQrStart: (source) => call('source_qr_start', { source }),
    sourceQrCheck: (source, key) => call('source_qr_check', { source, key }),
    account: () => call('account'),
    setCookie: (musicU) => call('set_cookie', { musicU }),
    logout: () => call('logout'),
    qrStart: () => call('qr_start'),
    qrCheck: (key) => call('qr_check', { key }),
    addJob: (job) => call('add_job', { job }),
    jobs: () => call('jobs'),
    job: (id) => call('job', { id }),
    jobAction: (id, action) => call('job_action', { id, action }),
    removeJob: (id) => call('remove_job', { id }),
    clearJobs: () => call('clear_jobs'),
    setConcurrency: (value) => call('set_concurrency', { value }),
    onJobs(cb) {
      let un: (() => void) | null = null;
      let closed = false;
      listen<JobSummary[]>('jobs', (e) => cb(e.payload)).then((f) => {
        if (closed) f();
        else un = f;
      });
      call<JobSummary[]>('jobs').then(cb);
      return () => {
        closed = true;
        un?.();
      };
    },
    reveal: (job) => call('reveal', { id: job.id }),
    async pickDir() {
      const { open } = await import('@tauri-apps/plugin-dialog');
      const r = await open({ directory: true, multiple: false });
      return typeof r === 'string' ? r : null;
    },
    defaultDir: () => call('default_dir_cmd'),
    library: () => call('library'),
    toggleLike: (track, alts = []) => call('toggle_like', { track, alts }),
    createPlaylist: (name) => call('create_playlist', { name }),
    playlistAdd: (id, items) => call('playlist_add', { id, items }),
    playlistUpdate: (id, patch) => call('playlist_update', { id, name: patch.name ?? null, remove: patch.remove ?? null }),
    playlistDelete: (id) => call('playlist_delete', { id }),
    recordPlay: (track, alts = []) => call('record_play', { track, alts }),
    clearPlays: () => call('clear_plays'),
  };
}

export const api: Api = isTauri ? await tauriApi() : httpApi;
