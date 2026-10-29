<script lang="ts">
  import { emit, listen } from '@tauri-apps/api/event';
  import { invoke } from '@tauri-apps/api/core';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import Icon from '../components/Icon.svelte';
  import { fade } from '../lib/fade';
  import { img } from '../lib/format';
  import { EV_CONTROL, EV_STATE, type BridgeControl, type BridgeState } from '../lib/bridge';

  let s = $state<BridgeState | null>(null);
  let pinned = $state(localStorage.getItem('shiyin.mini.pin') !== '0');
  let showLyric = $state(false);

  const ctl = (c: BridgeControl) => emit(EV_CONTROL, c);

  $effect(() => {
    const un = listen<BridgeState>(EV_STATE, (e) => (s = e.payload));
    ctl({ type: 'hello' });
    return () => void un.then((f) => f());
  });

  $effect(() => {
    localStorage.setItem('shiyin.mini.pin', pinned ? '1' : '0');
    getCurrentWindow().setAlwaysOnTop(pinned);
  });

  // 进度条：根据最近一次同步推算
  let ratio = $state(0);
  $effect(() => {
    const st = s;
    if (!st) return;
    let raf = 0;
    const tick = () => {
      const now = st.playing ? st.at + (Date.now() - st.sentAt) / 1000 : st.at;
      ratio = st.duration > 0 ? Math.min(1, now / st.duration) : 0;
      if (st.playing) raf = requestAnimationFrame(tick);
    };
    tick();
    return () => cancelAnimationFrame(raf);
  });

  function drag(e: PointerEvent) {
    if (e.button !== 0 || (e.target as HTMLElement).closest('button')) return;
    getCurrentWindow().startDragging();
  }

  const t = $derived(s?.track);
</script>

<div class="mini" style:--tint={s?.tint || '128 128 136'} onpointerdown={drag} role="presentation">
  <div class="wash"></div>

  <button class="cover" ondblclick={() => ctl({ type: 'show-main' })} title="双击回到主窗口" aria-label="回到主窗口">
    {#if t?.cover}
      {#key t.id}<img use:fade src={img(t.cover, 64)} alt="" width="64" height="64" />{/key}
    {/if}
  </button>

  <div class="meta">
    {#key showLyric ? s?.line : t?.id}
      <div class="swap">
        {#if showLyric && s?.line}
          <div class="name ellipsis">{s.line}</div>
          <div class="sub ellipsis">{s.trans || s.next}</div>
        {:else}
          <div class="name ellipsis">{t?.name ?? '未在播放'}</div>
          <div class="sub ellipsis">{t?.artist ?? '拾音'}</div>
        {/if}
      </div>
    {/key}
  </div>

  <div class="ctl">
    <button onclick={() => ctl({ type: 'prev' })} aria-label="上一首"><Icon name="prev" /></button>
    <button class="play" onclick={() => ctl({ type: 'toggle' })} aria-label="播放或暂停">
      <Icon name={s?.playing ? 'pause' : 'play'} />
    </button>
    <button onclick={() => ctl({ type: 'next' })} aria-label="下一首"><Icon name="next" /></button>
  </div>

  <div class="corner">
    <button class:on={showLyric} onclick={() => (showLyric = !showLyric)} title="显示歌词" aria-label="显示歌词"><Icon name="lyric" /></button>
    <button class:on={pinned} onclick={() => (pinned = !pinned)} title="置顶" aria-label="置顶"><Icon name="lock" /></button>
    <button onclick={() => ctl({ type: 'show-main' })} title="回到主窗口" aria-label="回到主窗口"><Icon name="expand" /></button>
    <button onclick={() => invoke('set_mini_window', { on: false })} title="关闭" aria-label="关闭"><Icon name="close" /></button>
  </div>

  <div class="progress"><div class="fill" style:transform="scaleX({ratio})"></div></div>
</div>

<style>
  :global(html),
  :global(body) {
    background: transparent !important;
    overflow: hidden;
  }
  .mini {
    position: fixed;
    inset: 0;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 0 12px;
    border-radius: 14px;
    overflow: hidden;
    background: var(--bg-elev);
    box-shadow: inset 0 0 0 0.5px var(--line-2);
    user-select: none;
    -webkit-user-select: none;
    animation: mini-in var(--t-slow) var(--ease) both;
  }
  @keyframes mini-in {
    from {
      opacity: 0;
      transform: scale(0.96);
    }
  }
  /* 封面主色的极淡底色，切歌时平滑过渡 */
  .wash {
    position: absolute;
    inset: 0;
    background: linear-gradient(100deg, rgb(var(--tint) / 0.16), transparent 60%);
    transition: background var(--t-slow) var(--ease);
    pointer-events: none;
  }
  .cover {
    position: relative;
    width: 64px;
    height: 64px;
    flex: none;
    border-radius: 8px;
    overflow: hidden;
    background: var(--bg-3);
    box-shadow: var(--shadow-sm);
  }
  .cover img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .meta {
    position: relative;
    flex: 1;
    min-width: 0;
  }
  .swap {
    animation: rise var(--t) var(--ease-out) backwards;
  }
  .name {
    font-size: 14px;
    font-weight: 600;
    letter-spacing: -0.01em;
  }
  .sub {
    margin-top: 1px;
    font-size: 12px;
    color: var(--fg-2);
  }
  .ctl {
    position: relative;
    display: flex;
    align-items: center;
    gap: 2px;
    transition: opacity var(--t) var(--ease);
  }
  .ctl button {
    width: 32px;
    height: 32px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    color: var(--fg);
    transition:
      background-color var(--t-fast) var(--ease),
      transform var(--t-fast) var(--ease);
  }
  .ctl button:hover {
    background: var(--hover);
  }
  .ctl button:active {
    transform: scale(0.9);
  }
  .ctl :global(svg) {
    width: 18px;
    height: 18px;
  }
  .ctl .play :global(svg) {
    width: 22px;
    height: 22px;
  }
  /* 悬停时右上角浮出窗口按钮，控制按钮让位 */
  .corner {
    position: absolute;
    top: 6px;
    right: 8px;
    display: flex;
    gap: 2px;
    opacity: 0;
    transform: translateY(-3px);
    transition:
      opacity var(--t) var(--ease),
      transform var(--t) var(--ease);
  }
  .mini:hover .corner {
    opacity: 1;
    transform: none;
  }
  .mini:hover .ctl {
    opacity: 0.35;
  }
  .mini:hover .ctl:hover {
    opacity: 1;
  }
  .corner button {
    width: 22px;
    height: 22px;
    border-radius: 6px;
    display: grid;
    place-items: center;
    color: var(--fg-3);
    transition:
      background-color var(--t-fast) var(--ease),
      color var(--t-fast) var(--ease);
  }
  .corner button:hover {
    background: var(--hover);
    color: var(--fg);
  }
  .corner button.on {
    color: var(--accent);
  }
  .corner :global(svg) {
    width: 13px;
    height: 13px;
  }
  .progress {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    height: 2px;
    background: var(--line);
  }
  .fill {
    height: 100%;
    background: rgb(var(--tint));
    transform-origin: left;
  }
</style>
