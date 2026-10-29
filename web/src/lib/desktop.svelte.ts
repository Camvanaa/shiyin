// 桌面版专属：托盘、桌面歌词、迷你播放器、全局快捷键。Web 版不加载。
import { emit, listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { player } from './player.svelte';
import { settings } from './settings.svelte';
import { lineAt } from './lrc';
import { artists } from './format';
import { EV_CONTROL, EV_STATE, type BridgeControl, type BridgeState } from './bridge';

export const desktop = $state({
  lyrics: false,
  lyricsLocked: false,
  mini: false,
});

let lastKey = '';
let lastSent = 0;

function snapshot(): BridgeState {
  const t = player.current;
  const lines = player.lyric;
  const ms = player.time * 1000 + 150;
  const i = lineAt(lines, ms);
  const cur = i >= 0 ? lines[i] : null;
  const nxt = lines[i + 1];
  return {
    track: t ? { id: t.id, name: t.name, artist: artists(t), cover: t.cover } : null,
    playing: player.playing,
    at: player.time,
    sentAt: Date.now(),
    duration: player.duration,
    line: cur?.text ?? (t ? t.name : ''),
    trans: cur?.trans ?? '',
    next: nxt?.text ?? '',
    lineStart: cur ? cur.t / 1000 : 0,
    lineEnd: nxt ? nxt.t / 1000 : player.duration,
    tint: player.tint,
  };
}

/** 只在关键状态变化时广播；进度由子窗口自行推算，另外每 5 秒校准一次 */
function broadcast(force = false) {
  if (!desktop.lyrics && !desktop.mini) return;
  const s = snapshot();
  const key = `${s.track?.id}|${s.playing}|${s.line}|${s.tint}|${Math.round(s.duration)}`;
  const now = Date.now();
  if (!force && key === lastKey && now - lastSent < 5000) return;
  lastKey = key;
  lastSent = now;
  emit(EV_STATE, s);
}

export async function initDesktop() {
  $effect.root(() => {
    $effect(() => {
      invoke('set_desktop_prefs', {
        prefs: { closeToTray: settings.closeToTray, globalShortcuts: settings.globalShortcuts },
      }).catch(() => {});
    });
    $effect(() => {
      // 读取这些值以建立依赖
      void player.current;
      void player.playing;
      void player.time;
      void player.lyric;
      void player.tint;
      broadcast();
    });
  });

  await listen<BridgeControl>(EV_CONTROL, (e) => {
    const c = e.payload;
    if (c.type === 'toggle') player.toggle();
    else if (c.type === 'next') player.next();
    else if (c.type === 'prev') player.prev();
    else if (c.type === 'seek') player.seek(c.value);
    else if (c.type === 'show-main') setMini(false);
    else if (c.type === 'hello') broadcast(true);
  });

  // 托盘菜单和全局快捷键由 Rust 转发到这里
  await listen<string>('tray', (e) => {
    const a = e.payload;
    if (a === 'toggle') player.toggle();
    else if (a === 'next') player.next();
    else if (a === 'prev') player.prev();
    else if (a === 'lyrics') setLyrics(!desktop.lyrics);
    else if (a === 'lock') setLyricsLocked(!desktop.lyricsLocked);
    else if (a === 'mini') setMini(!desktop.mini);
  });
  await listen<{ lyrics: boolean; locked: boolean; mini: boolean }>('windows', (e) => {
    desktop.lyrics = e.payload.lyrics;
    desktop.lyricsLocked = e.payload.locked;
    desktop.mini = e.payload.mini;
  });
}

export async function setLyrics(on: boolean) {
  desktop.lyrics = on;
  await invoke('set_lyrics_window', { on });
  if (on) setTimeout(() => broadcast(true), 300);
}

export async function setLyricsLocked(locked: boolean) {
  desktop.lyricsLocked = locked;
  await invoke('set_lyrics_locked', { locked });
}

export async function setMini(on: boolean) {
  desktop.mini = on;
  await invoke('set_mini_window', { on });
  if (on) setTimeout(() => broadcast(true), 300);
}
