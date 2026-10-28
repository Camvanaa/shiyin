export interface HistoryEntry {
  /** 用户输入的原文 */
  q: string;
  /** 显示标题：搜索词，或解析出的歌曲 / 歌单 / 专辑名 */
  title: string;
  kind: 'query' | 'song' | 'playlist' | 'album';
  cover?: string;
  at: number;
}

const KEY = 'shiyin.history';
const MAX = 30;

function load(): HistoryEntry[] {
  try {
    return JSON.parse(localStorage.getItem(KEY) ?? '[]');
  } catch {
    return [];
  }
}

export const history = $state<{ items: HistoryEntry[] }>({ items: load() });

function save() {
  localStorage.setItem(KEY, JSON.stringify(history.items));
}

export function remember(e: Omit<HistoryEntry, 'at'>) {
  const key = e.kind === 'query' ? e.q.trim().toLowerCase() : `${e.kind}:${e.title}`;
  history.items = [
    { ...e, at: Date.now() },
    ...history.items.filter((x) => (x.kind === 'query' ? x.q.trim().toLowerCase() : `${x.kind}:${x.title}`) !== key),
  ].slice(0, MAX);
  save();
}

export function forget(at: number) {
  history.items = history.items.filter((x) => x.at !== at);
  save();
}

export function clearHistory() {
  history.items = [];
  save();
}

/** 简单的前缀 / 包含匹配，前缀优先 */
export function suggest(input: string, limit = 6): HistoryEntry[] {
  const s = input.trim().toLowerCase();
  if (!s) return history.items.slice(0, limit);
  const hits = history.items.filter((x) => x.title.toLowerCase().includes(s) || x.q.toLowerCase().includes(s));
  hits.sort((a, b) => Number(b.title.toLowerCase().startsWith(s)) - Number(a.title.toLowerCase().startsWith(s)));
  return hits.slice(0, limit);
}
