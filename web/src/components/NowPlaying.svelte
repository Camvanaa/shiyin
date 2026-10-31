<script lang="ts">
  import { fade } from '../lib/fade';
  import Icon from './Icon.svelte';
  import Slider from './Slider.svelte';
  import { player } from '../lib/player.svelte';
  import { app } from '../lib/app.svelte';
  import { artists, duration, img, sourceName } from '../lib/format';
  import { lineAt } from '../lib/lrc';
  import { LEVELS } from '../lib/types';

  const t = $derived(player.current);
  let active = $state(-1);
  let lyricBox = $state<HTMLElement>();
  let userScrolling = $state(false);
  let scrollTimer: ReturnType<typeof setTimeout> | undefined;
  let scrub = $state<number | null>(null);
  /** 移动端：封面 / 歌词 二选一 */
  let mobileLyrics = $state(false);

  // 歌词行提前 0.15s 切换，视觉上更跟手。用 frameTime 保证逐帧精度。
  $effect(() => {
    const i = player.expanded ? lineAt(player.lyric, player.frameTime * 1000 + 150) : -1;
    if (i !== active) active = i;
  });

  // 当前行停在容器 42% 高度处，整列表平移
  let offset = $state(0);
  let boxH = $state(0);
  $effect(() => {
    const i = active;
    void boxH;
    void player.lyric;
    void mobileLyrics;
    if (!lyricBox || userScrolling) return;
    const el = lyricBox.querySelector<HTMLElement>(`[data-i="${Math.max(0, i)}"]`);
    if (!el) return;
    offset = lyricBox.clientHeight * 0.42 - (el.offsetTop + el.offsetHeight / 2);
  });

  function freeScroll(delta: number) {
    userScrolling = true;
    offset -= delta;
    clearTimeout(scrollTimer);
    scrollTimer = setTimeout(() => (userScrolling = false), 2500);
  }
  function onWheel(e: WheelEvent) {
    freeScroll(e.deltaY);
  }
  let touchY = 0;
  function onTouchStart(e: TouchEvent) {
    touchY = e.touches[0].clientY;
  }
  function onTouchMove(e: TouchEvent) {
    const y = e.touches[0].clientY;
    freeScroll(touchY - y);
    touchY = y;
  }

  function close() {
    player.expanded = false;
    mobileLyrics = false;
  }
  function onKey(e: KeyboardEvent) {
    if (e.key === 'Escape' && player.expanded) close();
  }

  // 下滑关闭手势（仅在非歌词滚动区域触发）
  let dragY = $state(0);
  let dragging = $state(false);
  let startY = 0;
  function dragStart(e: TouchEvent) {
    if ((e.target as HTMLElement).closest('.lyric, .slider, button')) return;
    startY = e.touches[0].clientY;
    dragging = true;
  }
  function dragMove(e: TouchEvent) {
    if (!dragging) return;
    dragY = Math.max(0, e.touches[0].clientY - startY);
  }
  function dragEnd() {
    if (!dragging) return;
    dragging = false;
    if (dragY > 120) close();
    dragY = 0;
  }

  const levelLabel = $derived(LEVELS.find((l) => l.value === player.quality)?.label ?? '');
  const remaining = $derived(Math.max(0, player.duration - (scrub ?? player.time)));
</script>

<svelte:window onkeydown={onKey} />

{#if t && player.expanded}
  <div
    class="np"
    class:dragging
    class:lyrics-mode={mobileLyrics}
    style:--tint={player.tint || '120 120 128'}
    style:transform={dragY ? `translateY(${dragY}px)` : ''}
    role="dialog"
    aria-modal="true"
    aria-label="正在播放"
    tabindex="-1"
    ontouchstart={dragStart}
    ontouchmove={dragMove}
    ontouchend={dragEnd}
    ontouchcancel={dragEnd}
  >
    <div class="bg"></div>

    <header>
      <button class="icon-btn" onclick={close} aria-label="收起"><Icon name="chevronDown" stroke={2.2} /></button>
      <div class="grabber" aria-hidden="true"></div>
      <div class="tags">
        <span class="q">{player.matchedFrom ? `${sourceName(t.source)} → ${sourceName(player.matchedFrom)}` : sourceName(t.source)}</span>
        {#if player.trial}<span class="q">试听</span>{/if}
        {#if levelLabel}<span class="q">{levelLabel}</span>{/if}
      </div>
      <button class="icon-btn" onclick={() => (app.sheet = 'queue')} aria-label="播放列表"><Icon name="queue" /></button>
    </header>

    <div class="body">
      <!-- 左栏（桌面）/ 下方控制区（移动） -->
      <div class="left">
        {#key t.id}
          <div class="art" class:paused={!player.playing}>
            <button class="art-btn" onclick={() => (mobileLyrics = true)} aria-label="查看歌词">
              <img use:fade src={img(t.cover, 400)} alt="" decoding="async" />
            </button>
          </div>
        {/key}

        <div class="meta">
          {#key t.id}
            <div class="swap">
              <div class="name ellipsis">{t.name}</div>
              <div class="artist ellipsis">{artists(t)}{#if t.album}{' — '}{t.album}{/if}</div>
            </div>
          {/key}
        </div>

        <div class="progress">
          <Slider label="播放进度" value={player.time} max={player.duration || 1} buffered={player.buffered} oninput={(v) => (scrub = v)} onchange={(v) => player.seek(v)} />
          <div class="times tabular">
            <span>{duration((scrub ?? player.time) * 1000)}</span>
            <span>-{duration(remaining * 1000)}</span>
          </div>
        </div>

        <div class="controls">
          <button class="icon-btn aux" class:on={player.shuffle} onclick={() => (player.shuffle = !player.shuffle)} aria-label="随机播放"><Icon name="shuffle" /></button>
          <button class="ctl" onclick={() => player.prev()} aria-label="上一首"><Icon name="prev" /></button>
          <button class="ctl big" onclick={() => player.toggle()} aria-label={player.playing ? '暂停' : '播放'}>
            {#if player.loading}<span class="spinner"></span>{:else}<Icon name={player.playing ? 'pause' : 'play'} />{/if}
          </button>
          <button class="ctl" onclick={() => player.next()} aria-label="下一首"><Icon name="next" /></button>
          <button class="icon-btn aux rep" class:on={player.repeat !== 'off'} onclick={() => player.cycleRepeat()} aria-label="循环模式">
            <Icon name="repeat" />{#if player.repeat === 'one'}<span class="one">1</span>{/if}
          </button>
        </div>

        <div class="m-tools">
          <button class="tool" class:on={mobileLyrics} onclick={() => (mobileLyrics = !mobileLyrics)} aria-pressed={mobileLyrics} aria-label="歌词">
            <Icon name="lyric" size={19} />
          </button>
          <button class="tool" onclick={() => (app.sheet = 'queue')} aria-label="播放列表">
            <Icon name="queue" size={19} />
          </button>
        </div>
      </div>

      <!-- 歌词：桌面在右栏，移动端在歌词模式时占据中部 -->
      <div class="lyric" role="region" aria-label="歌词" bind:this={lyricBox} bind:clientHeight={boxH} onwheel={onWheel} ontouchstart={onTouchStart} ontouchmove={onTouchMove}>
        {#if player.lyric.length}
          <div class="lines" class:free={userScrolling} style:transform="translateY({offset}px)">
            {#each player.lyric as l, i (l.t + ':' + i)}
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
    z-index: 95;
    display: flex;
    flex-direction: column;
    color: #fff;
    background: #111;
    animation: np-in var(--t-sheet) var(--ease) both;
    user-select: none;
    -webkit-user-select: none;
    will-change: transform;
    transition: transform var(--t) var(--ease);
  }
  .np.dragging {
    transition: none;
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
      radial-gradient(80% 70% at 18% 28%, rgb(var(--tint) / 0.95), transparent 72%),
      radial-gradient(70% 70% at 88% 85%, rgb(var(--tint) / 0.6), transparent 70%),
      radial-gradient(60% 60% at 80% 10%, rgb(var(--tint) / 0.35), transparent 70%),
      #101010;
    filter: saturate(1.6) brightness(0.9);
    transition: background var(--t-slow) var(--ease);
    pointer-events: none;
  }
  .bg::after {
    content: '';
    position: absolute;
    inset: 0;
    background: linear-gradient(rgb(0 0 0 / 0.1), rgb(0 0 0 / 0.45));
  }

  header {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 56px;
    padding: 0 16px;
    flex: none;
  }
  header .icon-btn {
    color: rgb(255 255 255 / 0.8);
  }
  header .icon-btn:hover {
    background: rgb(255 255 255 / 0.12);
    color: #fff;
  }
  .grabber {
    display: none;
  }
  .tags {
    display: flex;
    gap: 6px;
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
    transition: transform 600ms var(--ease-spring), box-shadow 600ms var(--ease);
    animation: fade var(--t-slow) var(--ease);
  }
  .art.paused {
    transform: scale(0.86);
    box-shadow: 0 12px 32px rgb(0 0 0 / 0.35);
  }
  .art-btn {
    display: block;
    width: 100%;
    height: 100%;
    pointer-events: none;
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
  .progress :global(.slider:hover .fill),
  .progress :global(.dragging .fill) {
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
  .icon-btn.aux {
    color: rgb(255 255 255 / 0.6);
    position: relative;
  }
  .icon-btn.aux.on {
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
    transition: transform var(--t-fast) var(--ease), background-color var(--t-fast) var(--ease);
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
  .ctl .spinner {
    width: 24px;
    height: 24px;
    border-color: rgb(255 255 255 / 0.25);
    border-top-color: #fff;
  }
  .m-tools {
    display: none;
  }

  .lyric {
    position: relative;
    height: 100%;
    max-height: 78vh;
    overflow: hidden;
    mask-image: linear-gradient(transparent, #000 12%, #000 82%, transparent);
    contain: layout paint;
  }
  .lines {
    will-change: transform;
    transition: transform 650ms var(--ease);
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
    transition: color 500ms var(--ease), transform 600ms var(--ease);
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

  /* ---------- 移动端：单列、封面与歌词互斥 ---------- */
  @media (max-width: 860px) {
    header {
      height: 52px;
      padding: 0 10px;
    }
    header > .icon-btn:first-child {
      visibility: hidden;
    }
    .grabber {
      display: block;
      position: absolute;
      left: 50%;
      top: 8px;
      width: 36px;
      height: 5px;
      margin-left: -18px;
      border-radius: 99px;
      background: rgb(255 255 255 / 0.32);
    }
    .tags {
      position: absolute;
      left: 50%;
      top: 22px;
      transform: translateX(-50%);
    }

    .body {
      display: flex;
      flex-direction: column;
      padding: 0 24px calc(14px + var(--sab));
      gap: 0;
    }
    .left {
      flex: 1;
      min-height: 0;
      justify-content: flex-end;
    }
    .art {
      width: min(100%, 78vw, 360px);
      margin: auto;
      border-radius: 18px;
      box-shadow: 0 20px 50px rgb(0 0 0 / 0.5);
    }
    .art-btn {
      pointer-events: auto;
    }
    .lyrics-mode .art {
      display: none;
    }
    .meta {
      margin-top: 22px;
    }
    .name {
      font-size: 21px;
    }
    .artist {
      font-size: 15px;
    }
    .progress {
      margin-top: 16px;
    }
    .controls {
      margin-top: 8px;
      padding: 0 2px;
    }
    .ctl {
      width: 60px;
      height: 60px;
    }
    .ctl.big {
      width: 72px;
      height: 72px;
      background: rgb(255 255 255 / 0.14);
    }
    .ctl.big:active {
      background: rgb(255 255 255 / 0.22);
    }
    .ctl :global(svg) {
      width: 30px;
      height: 30px;
    }
    .ctl.big :global(svg) {
      width: 36px;
      height: 36px;
    }

    .m-tools {
      display: flex;
      justify-content: center;
      gap: 28px;
      margin-top: 12px;
    }
    .tool {
      width: 40px;
      height: 40px;
      border-radius: 50%;
      display: grid;
      place-items: center;
      color: rgb(255 255 255 / 0.55);
      transition: color var(--t-fast) var(--ease), background-color var(--t-fast) var(--ease);
    }
    .tool.on {
      color: #fff;
      background: rgb(255 255 255 / 0.16);
    }

    .lyric {
      display: none;
    }
    .lyrics-mode .lyric {
      display: block;
      order: -1;
      flex: 1;
      width: 100%;
      min-height: 0;
      max-height: none;
      margin: 0 0 6px;
      mask-image: linear-gradient(transparent, #000 8%, #000 86%, transparent);
    }
    .lyrics-mode .left {
      flex: none;
      width: 100%;
    }
    .lyrics-mode .meta {
      margin-top: 4px;
    }
    .lyrics-mode .name {
      font-size: 17px;
    }
    .lyrics-mode .artist {
      font-size: 13px;
    }
    .lyrics-mode .progress {
      margin-top: 10px;
    }
    .lyrics-mode .ctl {
      width: 50px;
      height: 50px;
    }
    .lyrics-mode .ctl.big {
      width: 60px;
      height: 60px;
    }
    .line {
      font-size: 24px;
      padding: 10px 4px;
      margin-left: 0;
      transform-origin: left center;
      transform: scale(0.96);
    }
    .line.on {
      transform: none;
    }
    .trans {
      font-size: 15px;
    }
  }
</style>
