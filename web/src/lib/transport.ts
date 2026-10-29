// 传输层：桌面版走 Tauri invoke，Web 版走 HTTP。界面代码只使用这里导出的 api。
import type {
  Account, Collection, HomeFeed, JobDetail, JobSummary, Lyric, NewJob, QrStart, QrState, Resolved, SongUrl, Track,
} from './types';

export const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

export interface Api {
  home(): Promise<HomeFeed>;
  resolve(q: string): Promise<Resolved>;
  search(q: string, limit: number, offset?: number): Promise<Track[]>;
  song(id: number): Promise<Track>;
  songUrl(id: number, level: string): Promise<SongUrl>;
  lyric(id: number): Promise<Lyric>;
  playlist(id: number): Promise<Collection>;
  album(id: number): Promise<Collection>;
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
  home: () => http('GET', '/home'),
  resolve: (q) => http('GET', `/resolve?q=${enc(q)}`),
  search: (q, limit, offset = 0) => http('GET', `/search?q=${enc(q)}&limit=${limit}&offset=${offset}`),
  song: (id) => http('GET', `/song/${id}`),
  songUrl: (id, level) => http('GET', `/url/${id}?level=${enc(level)}`),
  lyric: (id) => http('GET', `/lyric/${id}`),
  playlist: (id) => http('GET', `/playlist/${id}`),
  album: (id) => http('GET', `/album/${id}`),
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
    home: () => call('home'),
    resolve: (q) => call('resolve', { q }),
    search: (q, limit, offset = 0) => call('search', { q, limit, offset }),
    song: (id) => call('song', { id }),
    songUrl: (id, level) => call('song_url', { id, level }),
    lyric: (id) => call('lyric', { id }),
    playlist: (id) => call('playlist', { id }),
    album: (id) => call('album', { id }),
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
  };
}

export const api: Api = isTauri ? await tauriApi() : httpApi;
