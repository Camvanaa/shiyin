import { api, AuthError, isTauri } from './transport';
import { settings, downloadOptions } from './settings.svelte';
import { remember } from './history.svelte';
import { toast } from './toast.svelte';
import { artists } from './format';
import type { Account, Collection, JobSummary, Track } from './types';

export type View =
  | { kind: 'home' }
  | { kind: 'search'; q: string; tracks: Track[]; done: boolean }
  | { kind: 'song'; track: Track }
  | { kind: 'collection'; data: Collection };

export type Sheet = null | 'settings' | 'login' | 'downloads' | 'queue' | 'token';

class AppState {
  view = $state<View>({ kind: 'home' });
  loading = $state(false);
  error = $state('');
  sheet = $state<Sheet>(null);
  account = $state<Account | null>(null);
  hasCredential = $state(false);
  jobs = $state<JobSummary[]>([]);
  /** 视图切换计数，用于触发进入动画 */
  viewKey = $state(0);

  activeJobs = $derived(this.jobs.filter((j) => j.status === 'running' || j.status === 'waiting').length);

  #ctrl: AbortController | null = null;
  #backStack: View[] = [];
  canBack = $state(false);
  #unsubJobs: (() => void) | null = null;

  async init() {
    try {
      await this.refreshAccount();
      this.#startJobs();
      api.setConcurrency(settings.concurrency).catch(() => {});
    } catch (e) {
      if (e instanceof AuthError) this.sheet = 'token';
    }
  }

  #startJobs() {
    this.#unsubJobs?.();
    // 首次收到的列表只作为基线，不提示
    let prev: Map<number, JobSummary> | null = null;
    this.#unsubJobs = api.onJobs((jobs) => {
      if (prev) {
        for (const j of jobs) {
          const p = prev.get(j.id);
          if (!p) continue;
          // Web 版：文件就绪（ZIP 打包可能晚于状态变化）时交给浏览器保存
          const ready = isTauri ? j.status === 'done' : j.status === 'done' && j.hasArtifact;
          const wasReady = isTauri ? p.status === 'done' : p.status === 'done' && p.hasArtifact;
          if (ready && !wasReady) {
            toast.success(
              j.failed ? `「${j.title}」完成，${j.failed} 首失败` : `「${j.title}」下载完成`,
              undefined,
              { label: isTauri ? '在文件夹中显示' : '再次保存', run: () => api.reveal(j) },
            );
            if (!isTauri) api.reveal(j);
          } else if (j.status === 'failed' && p.status !== 'failed') {
            toast.error(`「${j.title}」下载失败`, '可在下载列表中查看原因并重试');
          }
        }
      }
      prev = new Map(jobs.map((j) => [j.id, j]));
      this.jobs = jobs.sort((a, b) => b.created - a.created);
    });
  }

  async refreshAccount() {
    const r = await api.account();
    this.account = r.account;
    this.hasCredential = r.hasCredential;
  }

  async logout() {
    await api.logout();
    this.account = null;
    this.hasCredential = false;
    toast.info('已退出登录');
  }

  #go(v: View, push = true) {
    if (push && this.view.kind !== 'home') this.#backStack.push($state.snapshot(this.view) as View);
    if (this.#backStack.length > 20) this.#backStack.shift();
    this.canBack = this.#backStack.length > 0;
    this.view = v;
    this.viewKey++;
    this.error = '';
  }

  back() {
    const v = this.#backStack.pop();
    this.canBack = this.#backStack.length > 0;
    if (v) {
      this.view = v;
      this.viewKey++;
    } else this.home();
  }

  home() {
    this.#ctrl?.abort();
    this.#backStack = [];
    this.canBack = false;
    this.loading = false;
    this.view = { kind: 'home' };
    this.viewKey++;
  }

  async #run<T>(fn: () => Promise<T>): Promise<T | undefined> {
    this.#ctrl?.abort();
    const ctrl = (this.#ctrl = new AbortController());
    this.loading = true;
    this.error = '';
    try {
      const r = await fn();
      if (ctrl.signal.aborted) return;
      return r;
    } catch (e) {
      if (ctrl.signal.aborted) return;
      if (e instanceof AuthError) this.sheet = 'token';
      this.error = (e as Error).message;
    } finally {
      if (this.#ctrl === ctrl) this.loading = false;
    }
  }

  /** 智能输入：链接 / ID / 关键词 */
  async submit(input: string) {
    const q = input.trim();
    if (!q) return;
    const r = await this.#run(() => api.resolve(q));
    if (!r) return;
    if (r.type === 'query') return this.search(r.value);
    if (r.type === 'song') return this.openSong(r.value, q);
    return this.openCollection(r.type, r.value, q);
  }

  async search(q: string) {
    const tracks = await this.#run(() => api.search(q, settings.searchLimit));
    if (!tracks) return;
    remember({ q, title: q, kind: 'query' });
    this.#go({ kind: 'search', q, tracks, done: tracks.length < settings.searchLimit });
  }

  async searchMore() {
    const v = this.view;
    if (v.kind !== 'search' || v.done || this.loading) return;
    this.loading = true;
    try {
      const more = await api.search(v.q, settings.searchLimit, v.tracks.length);
      const ids = new Set(v.tracks.map((t) => t.id));
      v.tracks.push(...more.filter((t) => !ids.has(t.id)));
      v.done = more.length < settings.searchLimit;
    } catch (e) {
      toast.error('加载更多失败', (e as Error).message);
    } finally {
      this.loading = false;
    }
  }

  async openSong(id: number, q = String(id)) {
    const track = await this.#run(() => api.song(id));
    if (!track) return;
    remember({ q, title: `${track.name} · ${artists(track)}`, kind: 'song', cover: track.cover });
    this.#go({ kind: 'song', track });
  }

  async openCollection(kind: 'playlist' | 'album', id: number, q = `${kind}:${id}`) {
    const data = await this.#run(() => (kind === 'playlist' ? api.playlist(id) : api.album(id)));
    if (!data) return;
    remember({ q, title: data.name, kind, cover: data.cover });
    this.#go({ kind: 'collection', data });
  }

  async download(tracks: Track[], title: string, opts: { subtitle?: string; cover?: string; collection?: boolean } = {}) {
    if (!tracks.length) return;
    let dir: string | undefined;
    if (isTauri) {
      dir = settings.dir || (await api.defaultDir?.()) || '';
      if (opts.collection && settings.subfolder) dir = joinPath(dir, sanitize(title));
    }
    try {
      await api.addJob({
        title,
        subtitle: opts.subtitle ?? '',
        cover: opts.cover ?? tracks[0].cover,
        items: tracks.map((track, i) => ({
          track,
          index: opts.collection ? i + 1 : null,
          total: opts.collection ? tracks.length : null,
        })),
        options: downloadOptions(),
        dir,
      });
      toast.info(tracks.length > 1 ? `已添加 ${tracks.length} 首到下载` : `已添加「${tracks[0].name}」到下载`);
    } catch (e) {
      toast.error('添加下载失败', (e as Error).message);
    }
  }
}

function sanitize(s: string) {
  return s.replace(/[\\/:*?"<>|]/g, '_').trim().slice(0, 120) || 'untitled';
}

function joinPath(a: string, b: string) {
  const sep = a.includes('\\') && !a.includes('/') ? '\\' : '/';
  return a.replace(/[\\/]+$/, '') + sep + b;
}

export const app = new AppState();
