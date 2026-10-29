<script lang="ts">
  import Icon from './Icon.svelte';
  import Slider from './Slider.svelte';
  import { player } from '../lib/player.svelte';
  import { app } from '../lib/app.svelte';
  import { artists, duration, img } from '../lib/format';
  import { isTauri } from '../lib/transport';

  // 桌面版功能按需加载，Web 版不引入
  let desk = $state<typeof import('../lib/desktop.svelte') | null>(null);
  if (isTauri) import('../lib/desktop.svelte').then((m) => (desk = m));

  const t = $derived(player.current);
  let scrub = $state<number | null>(null);
  let lastVol = 0.8;

  function toggleMute() {
    if (player.volume > 0) {
      lastVol = player.volume;
      player.volume = 0;
    } else player.volume = lastVol || 0.8;
  }

  function onWheelVol(e: WheelEvent) {
    e.preventDefault();
    player.volume = player.volume - Math.sign(e.deltaY) * 0.05;
  }
</script>

{#if t}
  <div class="bar" role="region" aria-label="播放控制">
    <div class="progress-top">
      <Slider
        thin
        label="播放进度"
        value={player.time}
        max={player.duration || 1}
        buffered={player.buffered}
        oninput={(v) => (scrub = v)}
        onchange={(v) => player.seek(v)}
      />
    </div>

    <div class="left">
      <button class="cover" onclick={() => (player.expanded = true)} aria-label="展开播放页">
        {#key t.id}
          <img src={img(t.cover, 48)} alt="" width="48" height="48" />
        {/key}
        <span class="expand"><Icon name="chevronDown" stroke={2.2} /></span>
      </button>
      <div class="meta">
        {#key t.id}
          <div class="swap">
            <button class="name ellipsis" onclick={() => app.openSong(t.id)}>{t.name}</button>
            <div class="artist ellipsis">{artists(t)}</div>
          </div>
        {/key}
      </div>
    </div>

    <div class="center">
      <button class="icon-btn sm" class:on={player.shuffle} onclick={() => (player.shuffle = !player.shuffle)} title="随机播放" aria-label="随机播放">
        <Icon name="shuffle" />
      </button>
      <button class="icon-btn" onclick={() => player.prev()} title="上一首" aria-label="上一首"><Icon name="prev" /></button>
      <button class="play" onclick={() => player.toggle()} aria-label={player.playing ? '暂停' : '播放'}>
        {#if player.loading}
          <span class="spinner"></span>
        {:else}
          <Icon name={player.playing ? 'pause' : 'play'} />
        {/if}
      </button>
      <button class="icon-btn" onclick={() => player.next()} title="下一首" aria-label="下一首"><Icon name="next" /></button>
      <button
        class="icon-btn sm"
        class:on={player.repeat !== 'off'}
        onclick={() => player.cycleRepeat()}
        title={player.repeat === 'one' ? '单曲循环' : player.repeat === 'all' ? '列表循环' : '不循环'}
        aria-label="循环模式"
      >
        <Icon name="repeat" />
        {#if player.repeat === 'one'}<span class="one">1</span>{/if}
      </button>
    </div>

    <div class="right">
      <span class="time tabular">{duration((scrub ?? player.time) * 1000)} / {duration(player.duration * 1000)}</span>
      <div class="vol" onwheel={onWheelVol}>
        <button class="icon-btn sm" onclick={toggleMute} aria-label="静音"><Icon name={player.volume === 0 ? 'mute' : 'volume'} /></button>
        <div class="vol-slider"><Slider label="音量" value={player.volume} onchange={(v) => (player.volume = v)} oninput={(v) => v !== null && (player.volume = v)} /></div>
      </div>
      {#if desk}
        <button class="icon-btn sm" class:on={desk.desktop.lyrics} onclick={() => desk!.setLyrics(!desk!.desktop.lyrics)} title="桌面歌词" aria-label="桌面歌词">
          <Icon name="subtitles" />
        </button>
        <button class="icon-btn sm" onclick={() => desk!.setMini(true)} title="迷你播放器" aria-label="迷你播放器">
          <Icon name="mini" />
        </button>
      {/if}
      <button class="icon-btn sm" class:on={player.expanded} onclick={() => (player.expanded = !player.expanded)} title="歌词" aria-label="歌词">
        <Icon name="lyric" />
      </button>
      <button class="icon-btn sm" class:on={app.sheet === 'queue'} onclick={() => (app.sheet = app.sheet === 'queue' ? null : 'queue')} title="播放列表" aria-label="播放列表">
        <Icon name="queue" />
      </button>
    </div>
  </div>
{/if}

<style>
  .bar {
    position: relative;
    height: var(--playbar);
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto minmax(0, 1fr);
    align-items: center;
    gap: 16px;
    padding: 0 16px;
    background: var(--bg);
    border-top: 0.5px solid var(--line);
    animation: bar-in var(--t-slow) var(--ease) both;
  }
  @keyframes bar-in {
    from {
      transform: translateY(100%);
    }
  }
  .progress-top {
    position: absolute;
    left: 0;
    right: 0;
    top: -8px;
    z-index: 2;
  }
  .progress-top :global(.track) {
    border-radius: 0;
  }
  .left {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
  }
  .cover {
    position: relative;
    width: 48px;
    height: 48px;
    flex: none;
    border-radius: var(--r-sm);
    overflow: hidden;
    background: var(--bg-3);
    box-shadow: var(--shadow-sm);
  }
  .cover img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    animation: fade var(--t) var(--ease);
  }
  .expand {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    background: rgba(0, 0, 0, 0.4);
    color: #fff;
    opacity: 0;
    transition: opacity var(--t-fast) var(--ease);
  }
  .expand :global(svg) {
    width: 20px;
    height: 20px;
    transform: rotate(180deg);
  }
  .cover:hover .expand {
    opacity: 1;
  }
  .meta {
    min-width: 0;
    overflow: hidden;
  }
  .swap {
    animation: rise var(--t) var(--ease-out);
  }
  .name {
    display: block;
    max-width: 100%;
    font-size: 14px;
    font-weight: 500;
    text-align: left;
  }
  .name:hover {
    text-decoration: underline;
    text-underline-offset: 2px;
    text-decoration-color: var(--fg-3);
  }
  .artist {
    font-size: 12px;
    color: var(--fg-2);
  }
  .center {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .center .icon-btn {
    color: var(--fg);
  }
  .center .icon-btn.sm {
    color: var(--fg-2);
    position: relative;
  }
  .center .icon-btn.sm.on {
    color: var(--accent);
  }
  .one {
    position: absolute;
    right: 5px;
    bottom: 5px;
    font-size: 8px;
    font-weight: 700;
  }
  .play {
    width: 40px;
    height: 40px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    background: var(--fg);
    color: var(--bg);
    transition: transform var(--t-fast) var(--ease);
  }
  .play:hover {
    transform: scale(1.05);
  }
  .play:active {
    transform: scale(0.94);
  }
  .play :global(svg) {
    width: 18px;
    height: 18px;
  }
  .play .spinner {
    border-color: color-mix(in srgb, var(--bg) 30%, transparent);
    border-top-color: var(--bg);
  }
  .right {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 4px;
  }
  .time {
    font-size: 12px;
    color: var(--fg-3);
    margin-right: 8px;
  }
  .vol {
    display: flex;
    align-items: center;
  }
  .vol-slider {
    width: 88px;
    margin-right: 6px;
  }
  .icon-btn.sm :global(svg) {
    width: 17px;
    height: 17px;
  }

  @media (max-width: 900px) {
    .time,
    .vol-slider {
      display: none;
    }
  }
  @media (max-width: 640px) {
    .bar {
      grid-template-columns: minmax(0, 1fr) auto;
    }
    .right {
      display: none;
    }
    .center .icon-btn.sm {
      display: none;
    }
  }
</style>
