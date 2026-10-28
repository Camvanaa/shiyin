<script lang="ts">
  import Icon from './Icon.svelte';
  import Slider from './Slider.svelte';
  import { player } from '../lib/player.svelte';
  import { app } from '../lib/app.svelte';
  import { artists, duration, img } from '../lib/format';
  import { lineAt } from '../lib/lrc';
  import { LEVELS } from '../lib/types';

  const t = $derived(player.current);
  let active = $state(-1);
  let lyricBox = $state<HTMLElement>();
  let userScrolling = $state(false);
  let scrollTimer: ReturnType<typeof setTimeout> | undefined;
  let scrub = $state<number | null>(null);
  let tint = $state('');

  // 逐帧读取播放时间，只在行变化时更新
  $effect(() => {
    const lines = player.lyric;
    let raf = 0;
    const tick = () => {
      // 歌词行稍微提前 0.15s 切换，视觉上更跟手
      const i = lineAt(lines, player.exactTime * 1000 + 150);
      if (i !== active) active = i;
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  });

  // 当前行居中，使用 transform 平移整个列表
  let offset = $state(0);
  $effect(() => {
    const i = active;
    if (!lyricBox || userScrolling) return;
    const el = lyricBox.querySelector<HTMLElement>(`[data-i="${Math.max(0, i)}"]`);
    if (!el) return;
    offset = lyricBox.clientHeight * 0.4 - (el.offsetTop + el.offsetHeight / 2);
  });

  function onWheel(e: WheelEvent) {
    userScrolling = true;
    offset -= e.deltaY;
    clearTimeout(scrollTimer);
    scrollTimer = setTimeout(() => (userScrolling = false), 2500);
  }

  // 从封面取一次平均色作为背景色调
  $effect(() => {
    const url = t?.cover;
    if (!url) return;
    const im = new Image();
    im.crossOrigin = 'anonymous';
    im.src = img(url, 32);
    im.onload = () => {
      try {
        const c = document.createElement('canvas');
        c.width = c.height = 8;
        const g = c.getContext('2d', { willReadFrequently: true })!;
        g.drawImage(im, 0, 0, 8, 8);
        const d = g.getImageData(0, 0, 8, 8).data;
        let r = 0, gg = 0, b = 0;
        for (let i = 0; i < d.length; i += 4) {
          r += d[i];
          gg += d[i + 1];
          b += d[i + 2];
        }
        const n = d.length / 4;
        tint = `${Math.round(r / n)} ${Math.round(gg / n)} ${Math.round(b / n)}`;
      } catch {
        tint = '';
      }
    };
  });

  function close() {
    player.expanded = false;
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === 'Escape' && player.expanded) close();
  }

  const levelLabel = $derived(LEVELS.find((l) => l.value === player.quality)?.label ?? '');
</script>

<svelte:window onkeydown={onKey} />

{#if t && player.expanded}
  <div class="np" style:--tint={tint || '120 120 128'} role="dialog" aria-label="正在播放">
    <div class="bg"></div>
    <header>
      <button class="icon-btn" onclick={close} aria-label="收起"><Icon name="chevronDown" stroke={2} /></button>
      {#if levelLabel}<span class="q">{levelLabel}</span>{/if}
      <button class="icon-btn" onclick={() => (app.sheet = 'queue')} aria-label="播放列表"><Icon name="queue" /></button>
    </header>

    <div class="body">
      <div class="left">
        {#key t.id}
          <div class="art" class:paused={!player.playing}>
            <img src={img(t.cover, 400)} alt="" />
          </div>
        {/key}
        <div class="meta">
          {#key t.id}
            <div class="swap">
              <div class="name ellipsis">{t.name}</div>
              <div class="artist ellipsis">{artists(t)} — {t.album}</div>
            </div>
          {/key}
        </div>
        <div class="progress">
          <Slider label="播放进度" value={player.time} max={player.duration || 1} buffered={player.buffered} oninput={(v) => (scrub = v)} onchange={(v) => player.seek(v)} />
          <div class="times tabular">
            <span>{duration((scrub ?? player.time) * 1000)}</span>
            <span>-{duration(Math.max(0, player.duration - (scrub ?? player.time)) * 1000)}</span>
          </div>
        </div>
        <div class="controls">
          <button class="icon-btn" class:on={player.shuffle} onclick={() => (player.shuffle = !player.shuffle)} aria-label="随机播放"><Icon name="shuffle" /></button>
          <button class="ctl" onclick={() => player.prev()} aria-label="上一首"><Icon name="prev" /></button>
          <button class="ctl big" onclick={() => player.toggle()} aria-label={player.playing ? '暂停' : '播放'}>
            <Icon name={player.playing ? 'pause' : 'play'} />
          </button>
          <button class="ctl" onclick={() => player.next()} aria-label="下一首"><Icon name="next" /></button>
          <button class="icon-btn rep" class:on={player.repeat !== 'off'} onclick={() => player.cycleRepeat()} aria-label="循环模式">
            <Icon name="repeat" />{#if player.repeat === 'one'}<span class="one">1</span>{/if}
          </button>
        </div>
      </div>

      <div class="lyric" bind:this={lyricBox} onwheel={onWheel}>
        {#if player.lyric.length}
          <div class="lines" class:free={userScrolling} style:transform="translateY({offset}px)">
            {#each player.lyric as l, i}
              <button
                class="line"
                class:on={i === active}
                class:near={Math.abs(i - active) === 1}
                data-i={i}
                onclick={() => {
                  player.seek(l.t / 1000);
                  userScrolling = false;
                }}
              >
                {l.text}
                {#if l.trans}<span class="trans">{l.trans}</span>{/if}
              </button>
            {/each}
          </div>
        {:else}
          <div class="empty">{player.loading ? '' : '纯音乐，或暂无歌词'}</div>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .np {
    position: fixed;
    inset: 0;
    z-index: 50;
    display: flex;
    flex-direction: column;
    color: #fff;
    background: #111;
    animation: np-in var(--t-slow) var(--ease) both;
  }
  @keyframes np-in {
    from {
      transform: translateY(100%);
    }
  }
  .bg {
    position: absolute;
    inset: 0;
    background:
      radial-gradient(120% 90% at 20% 30%, rgb(var(--tint) / 0.75), transparent 70%),
      radial-gradient(90% 80% at 85% 80%, rgb(var(--tint) / 0.45), transparent 70%),
      linear-gradient(rgb(var(--tint) / 0.25), rgb(0 0 0 / 0.6)),
      #141414;
    transition: background var(--t-slow) var(--ease);
    pointer-events: none;
  }
  .bg::after {
    content: '';
    position: absolute;
    inset: 0;
    background: rgb(0 0 0 / 0.28);
  }
  header {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 56px;
    padding: 0 16px;
  }
  header .icon-btn {
    color: rgb(255 255 255 / 0.8);
  }
  header .icon-btn:hover {
    background: rgb(255 255 255 / 0.12);
    color: #fff;
  }
  .q {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.04em;
    padding: 3px 8px;
    border-radius: 6px;
    background: rgb(255 255 255 / 0.14);
    color: rgb(255 255 255 / 0.85);
  }
  .body {
    position: relative;
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: minmax(280px, 420px) minmax(0, 1fr);
    gap: 7vw;
    align-items: center;
    padding: 0 7vw 6vh;
    max-width: 1400px;
    width: 100%;
    margin: 0 auto;
  }
  .left {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .art {
    aspect-ratio: 1;
    border-radius: var(--r-lg);
    overflow: hidden;
    background: rgb(255 255 255 / 0.08);
    box-shadow: 0 24px 60px rgb(0 0 0 / 0.45);
    transition: transform 600ms var(--ease);
    animation: fade var(--t-slow) var(--ease);
  }
  .art.paused {
    transform: scale(0.88);
  }
  .art img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .meta {
    margin-top: 28px;
    min-width: 0;
  }
  .swap {
    animation: rise var(--t) var(--ease-out);
  }
  .name {
    font: 600 22px/1.25 var(--font-display);
    letter-spacing: -0.02em;
  }
  .artist {
    margin-top: 2px;
    font-size: 16px;
    color: rgb(255 255 255 / 0.65);
  }
  .progress {
    margin-top: 18px;
  }
  .progress :global(.track) {
    background: rgb(255 255 255 / 0.2);
  }
  .progress :global(.buf) {
    background: rgb(255 255 255 / 0.12);
  }
  .progress :global(.fill) {
    background: rgb(255 255 255 / 0.75);
  }
  .progress :global(.slider:hover .fill) {
    background: #fff;
  }
  .times {
    display: flex;
    justify-content: space-between;
    margin-top: 4px;
    font-size: 11px;
    color: rgb(255 255 255 / 0.55);
  }
  .controls {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-top: 14px;
  }
  .controls .icon-btn {
    color: rgb(255 255 255 / 0.6);
    position: relative;
  }
  .controls .icon-btn:hover {
    background: rgb(255 255 255 / 0.1);
    color: #fff;
  }
  .controls .icon-btn.on {
    color: #fff;
  }
  .one {
    position: absolute;
    right: 5px;
    bottom: 5px;
    font-size: 8px;
    font-weight: 700;
  }
  .ctl {
    width: 52px;
    height: 52px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    color: #fff;
    transition:
      transform var(--t-fast) var(--ease),
      background-color var(--t-fast) var(--ease);
  }
  .ctl:hover {
    background: rgb(255 255 255 / 0.1);
  }
  .ctl:active {
    transform: scale(0.9);
  }
  .ctl :global(svg) {
    width: 28px;
    height: 28px;
  }
  .ctl.big :global(svg) {
    width: 36px;
    height: 36px;
  }

  .lyric {
    position: relative;
    height: 100%;
    max-height: 78vh;
    overflow: hidden;
    mask-image: linear-gradient(transparent, #000 14%, #000 80%, transparent);
  }
  .lines {
    will-change: transform;
    transition: transform 700ms var(--ease);
    padding: 0 0 50vh;
  }
  .lines.free {
    transition: transform 120ms linear;
  }
  .line {
    display: block;
    width: 100%;
    text-align: left;
    padding: 10px 12px;
    margin-left: -12px;
    border-radius: var(--r);
    font: 700 28px/1.3 var(--font-display);
    letter-spacing: -0.02em;
    color: rgb(255 255 255 / 0.32);
    transform-origin: left center;
    transform: scale(0.94);
    transition:
      color 500ms var(--ease),
      transform 600ms var(--ease);
  }
  .line:hover {
    background: rgb(255 255 255 / 0.06);
    color: rgb(255 255 255 / 0.55);
  }
  .line.near {
    color: rgb(255 255 255 / 0.42);
  }
  .line.on {
    color: #fff;
    transform: none;
  }
  .trans {
    display: block;
    margin-top: 4px;
    font-size: 18px;
    font-weight: 600;
  }
  .empty {
    height: 100%;
    display: grid;
    place-items: center;
    color: rgb(255 255 255 / 0.4);
    font-size: 17px;
  }

  @media (max-width: 860px) {
    .body {
      grid-template-columns: 1fr;
      grid-template-rows: auto 1fr;
      padding: 0 24px 24px;
      gap: 24px;
      align-items: start;
    }
    .art {
      max-width: 260px;
    }
    .line {
      font-size: 22px;
    }
  }
</style>
