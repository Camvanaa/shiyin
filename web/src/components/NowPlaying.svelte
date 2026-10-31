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
  let mobileMode = $state<'art' | 'lyrics'>('art');

  // 歌词行提前 0.15s 切换，视觉上更跟手。player.time 在播放时逐帧更新。
  $effect(() => {
    const i = player.expanded ? lineAt(player.lyric, player.time * 1000 + 150) : -1;
    if (i !== active) active = i;
  });

  // 当前行居中，使用 transform 平移整个列表
  let offset = $state(0);
  let boxH = $state(0);
  $effect(() => {
    const i = active;
    void boxH;
    void player.lyric;
    if (!lyricBox || userScrolling) return;
    const el = lyricBox.querySelector<HTMLElement>(`[data-i="${Math.max(0, i)}"]`);
    if (!el) return;
    offset = lyricBox.clientHeight * 0.45 - (el.offsetTop + el.offsetHeight / 2);
  });

  function onWheel(e: WheelEvent) {
    userScrolling = true;
    offset -= e.deltaY;
    clearTimeout(scrollTimer);
    scrollTimer = setTimeout(() => (userScrolling = false), 2500);
  }

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
  <div class="np" style:--tint={player.tint || '120 120 128'} role="dialog" aria-modal="true" aria-label="正在播放">
    <div class="bg"></div>

    <!-- 移动端顶部下拉横条抓手 -->
    <div class="grabber-wrap" onclick={close} role="button" tabindex="0" onkeydown={(e) => e.key === 'Enter' && close()} aria-label="收起播放器">
      <div class="grabber"></div>
    </div>

    <header>
      <button class="icon-btn" onclick={close} aria-label="收起"><Icon name="chevronDown" stroke={2.2} /></button>
      <div class="tags">
        <span class="q">{player.matchedFrom ? `${sourceName(t.source)} → ${sourceName(player.matchedFrom)}` : sourceName(t.source)}</span>
        {#if player.trial}<span class="q">试听</span>{/if}
        {#if levelLabel}<span class="q">{levelLabel}</span>{/if}
      </div>
      <button class="icon-btn" onclick={() => (app.sheet = 'queue')} aria-label="播放列表"><Icon name="queue" /></button>
    </header>

    <!-- 桌面端宽屏布局 -->
    <div class="body desktop-only">
      <div class="left">
        {#key t.id}
          <div class="art" class:paused={!player.playing}>
            <img use:fade src={img(t.cover, 400)} alt="" decoding="async" />
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

      <div class="lyric" bind:this={lyricBox} bind:clientHeight={boxH} onwheel={onWheel}>
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

    <!-- 移动端纵向一体化 Apple 播放界面 -->
    <div class="mobile-layout mobile-only">
      <!-- 中部核心：点击可在封面与歌词间切换 -->
      <div class="m-center">
        {#if mobileMode === 'art'}
          <div
            class="m-art-wrap"
            role="button"
            tabindex="0"
            onclick={() => (mobileMode = 'lyrics')}
            onkeydown={(e) => e.key === 'Enter' && (mobileMode = 'lyrics')}
            aria-label="点击切换为歌词"
          >
            {#key t.id}
              <div class="m-art" class:paused={!player.playing}>
                <img use:fade src={img(t.cover, 400)} alt="" decoding="async" />
              </div>
            {/key}
          </div>
        {:else}
          <div
            class="m-lyric-wrap"
            role="region"
            aria-label="歌词"
            bind:this={lyricBox}
            bind:clientHeight={boxH}
          >
            {#if player.lyric.length}
              <div class="lines" style:transform="translateY({offset}px)">
                {#each player.lyric as l, i}
                  <button
                    class="line"
                    class:on={i === active}
                    data-i={i}
                    onclick={() => player.seek(l.t / 1000)}
                  >
                    {l.text}
                    {#if l.trans}<span class="trans">{l.trans}</span>{/if}
                  </button>
                {/each}
              </div>
            {:else}
              <div class="empty">{player.loading ? '正在获取歌词…' : '纯音乐，或暂无歌词'}</div>
            {/if}
          </div>
        {/if}
      </div>

      <!-- 下部控制区：曲名、进度条、大按键 -->
      <div class="m-bottom">
        <div class="m-meta">
          <div class="name ellipsis">{t.name}</div>
          <div class="artist ellipsis">{artists(t)} — {t.album}</div>
        </div>

        <div class="progress">
          <Slider
            label="播放进度"
            value={player.time}
            max={player.duration || 1}
            buffered={player.buffered}
            oninput={(v) => (scrub = v)}
            onchange={(v) => player.seek(v)}
          />
          <div class="times tabular">
            <span>{duration((scrub ?? player.time) * 1000)}</span>
            <span>-{duration(Math.max(0, player.duration - (scrub ?? player.time)) * 1000)}</span>
          </div>
        </div>

        <div class="m-controls">
          <button class="m-ctl-sm" class:on={player.shuffle} onclick={() => (player.shuffle = !player.shuffle)} aria-label="随机播放">
            <Icon name="shuffle" size={20} />
          </button>
          <button class="m-ctl" onclick={() => player.prev()} aria-label="上一首">
            <Icon name="prev" size={28} />
          </button>
          <button class="m-ctl-play" onclick={() => player.toggle()} aria-label={player.playing ? '暂停' : '播放'}>
            <Icon name={player.playing ? 'pause' : 'play'} size={32} />
          </button>
          <button class="m-ctl" onclick={() => player.next()} aria-label="下一首">
            <Icon name="next" size={28} />
          </button>
          <button class="m-ctl-sm rep" class:on={player.repeat !== 'off'} onclick={() => player.cycleRepeat()} aria-label="循环模式">
            <Icon name="repeat" size={20} />
            {#if player.repeat === 'one'}<span class="one">1</span>{/if}
          </button>
        </div>

        <!-- 移动端底部切换：歌词 / 播放队列 -->
        <div class="m-sub-actions">
          <button
            class="m-sub-btn"
            class:active={mobileMode === 'lyrics'}
            onclick={() => (mobileMode = mobileMode === 'lyrics' ? 'art' : 'lyrics')}
            aria-label="切换歌词"
          >
            <Icon name="lyric" size={20} />
            <span>{mobileMode === 'lyrics' ? '封面' : '歌词'}</span>
          </button>

          <button
            class="m-sub-btn"
            onclick={() => (app.sheet = 'queue')}
            aria-label="播放队列"
          >
            <Icon name="queue" size={20} />
            <span>队列 ({player.queue.length})</span>
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  .np {
    position: fixed;
    inset: 0;
    z-index: 95; /* 覆盖主页面与底栏 */
    display: flex;
    flex-direction: column;
    color: #fff;
    background: #111;
    animation: np-in var(--t-slow) var(--ease) both;
    user-select: none;
    -webkit-user-select: none;
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

  .grabber-wrap {
    display: none;
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

  /* 桌面端双栏 */
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

  /* 移动端专属一体化排版 */
  .mobile-layout {
    position: relative;
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    padding: 0 20px calc(16px + var(--sab));
  }

  .m-center {
    flex: 1;
    min-height: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
  }

  .m-art-wrap {
    width: 100%;
    max-width: 320px;
    aspect-ratio: 1;
    cursor: pointer;
    outline: none;
  }
  .m-art {
    width: 100%;
    height: 100%;
    border-radius: 20px;
    overflow: hidden;
    background: rgb(255 255 255 / 0.08);
    box-shadow: 0 16px 48px rgba(0, 0, 0, 0.45);
    transition: transform 600ms var(--ease);
  }
  .m-art.paused {
    transform: scale(0.9);
  }
  .m-art img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .m-lyric-wrap {
    width: 100%;
    height: 100%;
    overflow: hidden;
    mask-image: linear-gradient(transparent, #000 12%, #000 85%, transparent);
    padding: 0 10px;
  }
  .m-lyric-wrap .line {
    font-size: 24px;
    padding: 12px 0;
    margin-left: 0;
  }

  .m-bottom {
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding-top: 10px;
  }

  .m-meta .name {
    font-size: 20px;
    font-weight: 700;
  }
  .m-meta .artist {
    font-size: 14px;
    color: rgb(255 255 255 / 0.65);
    margin-top: 2px;
  }

  .m-controls {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 4px;
  }
  .m-ctl-sm {
    width: 36px;
    height: 36px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    color: rgb(255 255 255 / 0.65);
    position: relative;
  }
  .m-ctl-sm.on {
    color: #fff;
  }
  .m-ctl {
    width: 48px;
    height: 48px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    color: #fff;
    transition: transform var(--t-fast) var(--ease);
  }
  .m-ctl:active {
    transform: scale(0.9);
  }
  .m-ctl-play {
    width: 64px;
    height: 64px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    background: #fff;
    color: #111;
    transition: transform var(--t-fast) var(--ease);
  }
  .m-ctl-play:active {
    transform: scale(0.92);
  }

  .m-sub-actions {
    display: flex;
    align-items: center;
    justify-content: space-around;
    padding-top: 6px;
  }
  .m-sub-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 14px;
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.1);
    color: rgb(255 255 255 / 0.7);
    font-size: 12px;
    font-weight: 500;
    transition: all var(--t-fast) var(--ease);
  }
  .m-sub-btn:active {
    transform: scale(0.95);
  }
  .m-sub-btn.active {
    background: #fff;
    color: #111;
  }

  @media (max-width: 860px) {
    .desktop-only {
      display: none !important;
    }
    .mobile-only {
      display: flex !important;
    }
    .grabber-wrap {
      display: flex;
      justify-content: center;
      padding: 10px 0 4px;
      cursor: pointer;
      z-index: 2;
    }
    .grabber {
      width: 38px;
      height: 4.5px;
      border-radius: 99px;
      background: rgba(255, 255, 255, 0.3);
    }
  }
  @media (min-width: 861px) {
    .mobile-only {
      display: none !important;
    }
  }
</style>
