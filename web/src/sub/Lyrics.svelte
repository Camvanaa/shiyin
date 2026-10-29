<script lang="ts">
  import { emit, listen } from '@tauri-apps/api/event';
  import { invoke } from '@tauri-apps/api/core';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import Icon from '../components/Icon.svelte';
  import { EV_CONTROL, EV_STATE, type BridgeControl, type BridgeState } from '../lib/bridge';

  let s = $state<BridgeState | null>(null);
  let locked = $state(false);
  let hover = $state(false);
  let size = $state(Number(localStorage.getItem('shiyin.lyrics.size') ?? 30));
  let showTrans = $state(localStorage.getItem('shiyin.lyrics.trans') !== '0');

  // 每次换行时递增，用于触发换行动画
  let gen = $state(0);
  let prevLine = '';

  const ctl = (c: BridgeControl) => emit(EV_CONTROL, c);

  $effect(() => {
    const un = listen<BridgeState>(EV_STATE, (e) => {
      const n = e.payload;
      if (n.line !== prevLine) {
        prevLine = n.line;
        gen++;
      }
      s = n;
    });
    const un2 = listen<{ locked: boolean }>('lyrics:locked', (e) => (locked = e.payload.locked));
    ctl({ type: 'hello' });
    return () => {
      un.then((f) => f());
      un2.then((f) => f());
    };
  });

  $effect(() => {
    localStorage.setItem('shiyin.lyrics.size', String(size));
    localStorage.setItem('shiyin.lyrics.trans', showTrans ? '1' : '0');
  });

  // 当前行的播放进度（0..1），用于底部细线与逐行扫光
  let progress = $state(0);
  $effect(() => {
    const st = s;
    if (!st) return;
    let raf = 0;
    const tick = () => {
      const now = st.playing ? st.at + (Date.now() - st.sentAt) / 1000 : st.at;
      const span = Math.max(0.3, st.lineEnd - st.lineStart);
      progress = Math.min(1, Math.max(0, (now - st.lineStart) / span));
      if (st.playing) raf = requestAnimationFrame(tick);
    };
    tick();
    return () => cancelAnimationFrame(raf);
  });

  function startDrag(e: PointerEvent) {
    if (locked || e.button !== 0) return;
    if ((e.target as HTMLElement).closest('button')) return;
    getCurrentWindow().startDragging();
  }

  async function lock() {
    locked = true;
    await invoke('set_lyrics_locked', { locked: true });
  }

  const sub = $derived(showTrans && s?.trans ? s.trans : (s?.next ?? ''));
</script>

<div
  class="root"
  class:hover={hover && !locked}
  class:locked
  style:--size="{size}px"
  style:--tint={s?.tint || '255 255 255'}
  onpointerenter={() => (hover = true)}
  onpointerleave={() => (hover = false)}
  onpointerdown={startDrag}
  role="presentation"
>
  <div class="bar" aria-hidden={!hover || locked}>
    <button onclick={() => ctl({ type: 'prev' })} aria-label="上一首"><Icon name="prev" /></button>
    <button onclick={() => ctl({ type: 'toggle' })} aria-label="播放或暂停"><Icon name={s?.playing ? 'pause' : 'play'} /></button>
    <button onclick={() => ctl({ type: 'next' })} aria-label="下一首"><Icon name="next" /></button>
    <span class="sep"></span>
    <button onclick={() => (size = Math.max(18, size - 2))} aria-label="缩小" class="txt">A<sup>−</sup></button>
    <button onclick={() => (size = Math.min(56, size + 2))} aria-label="放大" class="txt">A<sup>+</sup></button>
    <button class:on={showTrans} onclick={() => (showTrans = !showTrans)} aria-label="译文" class="txt">译</button>
    <span class="sep"></span>
    <button onclick={lock} aria-label="锁定"><Icon name="lock" /></button>
    <button onclick={() => invoke('set_lyrics_window', { on: false })} aria-label="关闭"><Icon name="close" /></button>
  </div>

  <div class="lines">
    {#key gen}
      <div class="line main">
        <span class="base">{s?.line || '拾音'}</span>
        <span class="lit" style:--p={progress} aria-hidden="true">{s?.line || '拾音'}</span>
      </div>
      {#if sub}<div class="line sub">{sub}</div>{/if}
    {/key}
  </div>
</div>

<style>
  :global(html),
  :global(body) {
    background: transparent !important;
    overflow: hidden;
  }
  .root {
    position: fixed;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 6px;
    border-radius: 16px;
    background: rgb(0 0 0 / 0);
    transition: background-color var(--t) var(--ease);
    user-select: none;
    -webkit-user-select: none;
  }
  .root.hover {
    background: rgb(20 20 22 / 0.55);
  }
  .bar {
    position: absolute;
    top: 8px;
    left: 50%;
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 3px;
    border-radius: 999px;
    background: rgb(255 255 255 / 0.1);
    opacity: 0;
    transform: translate(-50%, -4px);
    pointer-events: none;
    transition:
      opacity var(--t) var(--ease),
      transform var(--t) var(--ease);
  }
  .hover .bar {
    opacity: 1;
    transform: translate(-50%, 0);
    pointer-events: auto;
  }
  .bar button {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    color: rgb(255 255 255 / 0.82);
    transition:
      background-color var(--t-fast) var(--ease),
      transform var(--t-fast) var(--ease);
  }
  .bar button:hover {
    background: rgb(255 255 255 / 0.14);
    color: #fff;
  }
  .bar button:active {
    transform: scale(0.9);
  }
  .bar button.on {
    color: rgb(var(--tint));
  }
  .bar :global(svg) {
    width: 15px;
    height: 15px;
  }
  .txt {
    font-size: 13px;
    font-weight: 600;
  }
  .txt sup {
    font-size: 9px;
  }
  .sep {
    width: 1px;
    height: 14px;
    margin: 0 3px;
    background: rgb(255 255 255 / 0.16);
  }
  .lines {
    display: flex;
    flex-direction: column;
    align-items: center;
    max-width: 100%;
    padding: 0 24px;
  }
  .line {
    max-width: 100%;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    animation: line-in 420ms var(--ease-out) both;
  }
  @keyframes line-in {
    from {
      opacity: 0;
      transform: translateY(8px);
    }
  }
  .main {
    position: relative;
    font: 700 var(--size) / 1.25 var(--font-display);
    letter-spacing: -0.01em;
  }
  .base,
  .lit {
    display: block;
    /* 深色描边 + 柔和投影，在任何桌面背景上都清晰 */
    paint-order: stroke fill;
    -webkit-text-stroke: 3px rgb(0 0 0 / 0.35);
    text-shadow: 0 2px 10px rgb(0 0 0 / 0.35);
  }
  .base {
    color: rgb(255 255 255 / 0.92);
  }
  .lit {
    position: absolute;
    inset: 0;
    color: rgb(var(--tint));
    -webkit-text-stroke: 0;
    text-shadow: none;
    clip-path: inset(0 calc((1 - var(--p)) * 100%) 0 0);
  }
  .sub {
    font: 600 calc(var(--size) * 0.56) / 1.3 var(--font);
    color: rgb(255 255 255 / 0.72);
    paint-order: stroke fill;
    -webkit-text-stroke: 2px rgb(0 0 0 / 0.3);
    text-shadow: 0 1px 6px rgb(0 0 0 / 0.35);
    animation-delay: 40ms;
  }
</style>
