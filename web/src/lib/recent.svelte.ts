import type { Track } from './types';

/** 最近播放：真正开始播放时记录，按 source:id 去重，最新在前 */
const KEY = 'shiyin.recent';
const MAX = 100;

function load(): Track[] {
  try {
    const list = JSON.parse(localStorage.getItem(KEY) ?? '[]');
    return Array.isArray(list) ? list : [];
  } catch {
    return [];
  }
}

export const recent = $state<{ items: Track[] }>({ items: load() });

let timer: ReturnType<typeof setTimeout> | undefined;
function save() {
  clearTimeout(timer);
  timer = setTimeout(() => localStorage.setItem(KEY, JSON.stringify(recent.items)), 300);
}

export function recordPlay(t: Track) {
  const key = `${t.source}:${t.id}`;
  recent.items = [t, ...recent.items.filter((x) => `${x.source}:${x.id}` !== key)].slice(0, MAX);
  save();
}

export function removeRecent(t: Track) {
  const key = `${t.source}:${t.id}`;
  recent.items = recent.items.filter((x) => `${x.source}:${x.id}` !== key);
  save();
}

export function clearRecent() {
  recent.items = [];
  save();
}
