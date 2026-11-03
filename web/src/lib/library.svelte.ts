import { api } from './transport';
import { toast } from './toast.svelte';
import type { LibPlaylist, LibraryData, LibTrack, Track } from './types';

/** 资料库：喜欢、自建歌单、播放记录。数据在服务端/Rust 侧持久化，这里只做前端镜像。 */
class LibraryState {
  likes = $state<LibTrack[]>([]);
  playlists = $state<LibPlaylist[]>([]);
  plays = $state<LibraryData['plays']>([]);
  loaded = $state(false);
  #likeKeys = $state<Set<string>>(new Set());
  /** 记录 play 的节流：同一首 30 秒内不重复上报 */
  #lastPlayKey = '';
  #lastPlayAt = 0;

  get likeCount() {
    return this.likes.length;
  }

  isLiked(t: { id: string; source: string }): boolean {
    return this.#likeKeys.has(`${t.source}:${t.id}`);
  }

  async load() {
    try {
      const d = await api.library();
      this.likes = d.likes;
      this.playlists = d.playlists;
      this.plays = d.plays;
      this.#likeKeys = new Set(d.likes.map((t) => `${t.source}:${t.id}`));
      this.loaded = true;
    } catch {
      /* 未登录/网络问题时静默，资料库为空 */
    }
  }

  async toggleLike(track: Track) {
    const key = `${track.source}:${track.id}`;
    // 乐观更新
    const wasLiked = this.#likeKeys.has(key);
    const next = new Set(this.#likeKeys);
    if (wasLiked) next.delete(key);
    else next.add(key);
    this.#likeKeys = next;
    try {
      const r = await api.toggleLike(track, (track.alts ?? []).map((a) => `${a.source}:${a.id}`));
      if (r.liked !== !wasLiked) {
        // 服务端与预期不一致时以服务端为准
        const s = new Set(this.#likeKeys);
        if (r.liked) s.add(key);
        else s.delete(key);
        this.#likeKeys = s;
      }
      await this.load();
    } catch (e) {
      // 回滚
      const s = new Set(this.#likeKeys);
      if (wasLiked) s.add(key);
      else s.delete(key);
      this.#likeKeys = s;
      toast.error(r_likeError(wasLiked), (e as Error).message);
    }
  }

  /** 播放时上报（节流）。失败静默，不打扰播放。 */
  recordPlay(track: Track) {
    const key = `${track.source}:${track.id}`;
    const now = Date.now();
    if (key === this.#lastPlayKey && now - this.#lastPlayAt < 30_000) return;
    this.#lastPlayKey = key;
    this.#lastPlayAt = now;
    api.recordPlay(track, (track.alts ?? []).map((a) => `${a.source}:${a.id}`)).catch(() => {});
  }

  async createPlaylist(name: string): Promise<LibPlaylist | null> {
    try {
      const pl = await api.createPlaylist(name);
      this.playlists = [pl, ...this.playlists];
      return pl;
    } catch (e) {
      toast.error('创建歌单失败', (e as Error).message);
      return null;
    }
  }

  async addToPlaylist(id: string, tracks: Track[]): Promise<number> {
    try {
      const items = tracks.map((t) => ({ track: t, alts: (t.alts ?? []).map((a) => `${a.source}:${a.id}`) }));
      const r = await api.playlistAdd(id, items);
      await this.load();
      return r.added;
    } catch (e) {
      toast.error('加入歌单失败', (e as Error).message);
      return 0;
    }
  }

  async renamePlaylist(id: string, name: string) {
    await api.playlistUpdate(id, { name });
    this.playlists = this.playlists.map((p) => (p.id === id ? { ...p, name } : p));
  }

  async removeFromPlaylist(id: string, trackKey: string) {
    await api.playlistUpdate(id, { remove: [trackKey] });
    const pl = this.playlists.find((p) => p.id === id);
    if (pl) pl.tracks = pl.tracks.filter((t) => `${t.source}:${t.id}` !== trackKey);
  }

  async deletePlaylist(id: string) {
    await api.playlistDelete(id);
    this.playlists = this.playlists.filter((p) => p.id !== id);
  }

  async clearPlays() {
    await api.clearPlays();
    this.plays = [];
  }
}

function r_likeError(wasLiked: boolean) {
  return wasLiked ? '取消喜欢失败' : '喜欢失败';
}

export const library = new LibraryState();
