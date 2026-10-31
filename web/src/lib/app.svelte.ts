import { api, AuthError, isTauri } from './transport';
import { settings, downloadOptions } from './settings.svelte';
import { remember } from './history.svelte';
import { toast } from './toast.svelte';
import { artists } from './format';
import type { Account, Collection, HomeFeed, JobSummary, SourceInfo, Track } from './types';

export type View =
  | { kind: 'home' }
  /** 移动端的搜索入口页（历史、榜单快捷方式），不发起请求 */
  | { kind: 'search-home' }
  | { kind: 'search'; q: string; tracks: Track[]; done: boolean; offset: number }
  | { kind: 'song'; track: Track }
  | { kind: 'collection'; data: Collection };

export type Sheet = null | 'settings' | 'login' | 'downloads' | 'queue' | 'token';

export const SOURCE_TABS: readonly [string, string][] = [
  ['all', '全部'],
  ['netease', '网易云'],
  ['qq', 'QQ音乐'],
  ['kuwo', '酷我'],
];

class AppState {
  view = $state<View>({ kind: 'home' });
  loading = $state(false);
  error = $state('');
  sheet = $state<Sheet>(null);
  account = $state<Account | null>(null);
  hasCredential = $state(false);
  jobs = $state<JobSummary[]>([]);
  feed = $state<HomeFeed | null>(null);
  source = $state<string>(localStorage.getItem('shiyin.source') ?? 'all');
  homeLoading = $state(false);
  sources = $state<SourceInfo[]>([]);
  /** 视图切换计数，用于触发进入动画 */
  viewKey = $state(0);

  activeJobs = $derived(this.jobs.filter((j) => j.status === 'running' || j.status === 'waiting').length);

  #ctrl: AbortController | null = null;
  #backStack: View[] = [];
  canBack = $state(false);
  #unsubJobs: (() => void) | null = null;

  async init() {
    if (isTauri) import('./desktop.svelte').then((m) => m.initDesktop());
    try {
      api.sources().then((l) => (this.sources = l)).catch(() => {});
      this.loadHome();
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

  loadHome() {
    this.homeLoading = true;
    api
      .home(this.source)
      .then((f) => (this.feed = f))
      .catch(() => {})
      .finally(() => (this.homeLoading = false));
  }

  setSource(id: string) {
    if (this.source === id) return;
    this.source = id;
    localStorage.setItem('shiyin.source', id);
    this.loadHome();
    if (this.view.kind === 'search') this.search(this.view.q);
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
    const cur = this.view;
    // 搜索入口页是临时态，不进入返回栈
    if (push && cur.kind !== 'home' && cur.kind !== 'search-home') {
      this.#backStack.push($state.snapshot(cur) as View);
    }
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
    this.sheet = null;
    this.view = { kind: 'home' };
    this.viewKey++;
  }

  /** 移动端：进入搜索入口页 */
  openSearch() {
    this.sheet = null;
    if (this.view.kind === 'search' || this.view.kind === 'search-home') return;
    this.#go({ kind: 'search-home' });
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
    const m = q.match(/^(netease|qq|kuwo):(playlist|album|toplist|song):(.+)$/);
    if (m) {
      return m[2] === 'song'
        ? this.openSong(m[3], q, m[1])
        : this.openCollection(m[2] as 'playlist' | 'album' | 'toplist', m[3], q, m[1]);
    }
    const r = await this.#run(() => api.resolve(q));
    if (!r) return;
    if (r.type === 'query') return this.search(r.value);
    if (r.type === 'song') return this.openSong(r.id, q, r.source);
    return this.openCollection(r.type, r.id, q, r.source);
  }

  async search(q: string) {
    q = q.trim();
    if (!q) return;
    const tracks = await this.#run(() => api.search(q, settings.searchLimit, 0, this.source));
    if (!tracks) return;
    remember({ q, title: q, kind: 'query' });
    this.#go({ kind: 'search', q, tracks, done: tracks.length < settings.searchLimit, offset: tracks.length });
  }

  async searchMore() {
    const v = this.view;
    if (v.kind !== 'search' || v.done || this.loading) return;
    this.loading = true;
    try {
      const more = await api.search(v.q, settings.searchLimit, v.offset, this.source);
      v.offset += more.length;
      const ids = new Set(v.tracks.map((t) => `${t.source}:${t.id}`));
      const fresh = more.filter((t) => !ids.has(`${t.source}:${t.id}`));
      v.tracks.push(...fresh);
      // 聚合搜索各平台返回数可能不足 limit，用"是否有新内容"判断结束更可靠
      v.done = more.length === 0 || (fresh.length === 0 && more.length < settings.searchLimit);
    } catch (e) {
      toast.error('加载更多失败', (e as Error).message);
    } finally {
      this.loading = false;
    }
  }

  async openSong(id: string, q = String(id), source = 'netease') {
    const track = await this.#run(() => api.song(id, source));
    if (!track) return;
    remember({ q, title: `${track.name} · ${artists(track)}`, kind: 'song', cover: track.cover });
    this.#go({ kind: 'song', track });
  }

  async openCollection(kind: 'playlist' | 'album' | 'toplist', id: string, q?: string, source = 'netease') {
    const data = await this.#run(() =>
      kind === 'album' ? api.album(id, source) : kind === 'toplist' ? api.toplist(id, source) : api.playlist(id, source),
    );
    if (!data) return;
    remember({ q: q ?? `${source}:${kind}:${id}`, title: data.name, kind: kind === 'toplist' ? 'playlist' : kind, cover: data.cover });
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
