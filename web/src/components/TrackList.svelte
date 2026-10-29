<script lang="ts">
  import { fade } from '../lib/fade';
  import Icon from './Icon.svelte';
  import { player } from '../lib/player.svelte';
  import { app } from '../lib/app.svelte';
  import { artists, duration, img } from '../lib/format';
  import type { Track } from '../lib/types';

  let {
    tracks,
    scroller,
    selectable = false,
    selected = $bindable(new Set<number>()),
    showAlbum = true,
    showCover = true,
    numbered = false,
    onplay,
    onreachend,
  }: {
    tracks: Track[];
    /** 外层滚动容器，用于虚拟滚动 */
    scroller: HTMLElement | undefined;
    selectable?: boolean;
    selected?: Set<number>;
    showAlbum?: boolean;
    showCover?: boolean;
    numbered?: boolean;
    onplay: (index: number) => void;
    onreachend?: () => void;
  } = $props();

  const ROW = 56;
  const OVERSCAN = 10;

  let host = $state<HTMLElement>();
  let start = $state(0);
  let end = $state(40);

  const visible = $derived(tracks.slice(start, end));
  const currentId = $derived(player.current?.id);

  function measure() {
    if (!host || !scroller) return;
    const top = host.getBoundingClientRect().top - scroller.getBoundingClientRect().top;
    const viewTop = -top;
    const h = scroller.clientHeight;
    const s = Math.max(0, Math.floor(viewTop / ROW) - OVERSCAN);
    const e = Math.min(tracks.length, Math.ceil((viewTop + h) / ROW) + OVERSCAN);
    if (s !== start) start = s;
    if (e !== end) end = e;
    if (onreachend && e >= tracks.length - 4) onreachend();
  }

  $effect(() => {
    const el = scroller;
    if (!el) return;
    void tracks.length;
    let raf = 0;
    const onScroll = () => {
      if (raf) return;
      raf = requestAnimationFrame(() => {
        raf = 0;
        measure();
      });
    };
    measure();
    el.addEventListener('scroll', onScroll, { passive: true });
    const ro = new ResizeObserver(onScroll);
    ro.observe(el);
    return () => {
      el.removeEventListener('scroll', onScroll);
      ro.disconnect();
      cancelAnimationFrame(raf);
    };
  });

  function toggle(id: number) {
    const s = new Set(selected);
    if (s.has(id)) s.delete(id);
    else s.add(id);
    selected = s;
  }

  // 首屏行依次出现，滚动后渲染的行不再做动画
  let animate = $state(true);
  $effect(() => {
    void tracks;
    animate = true;
    const t = setTimeout(() => (animate = false), 600);
    return () => clearTimeout(t);
  });

  let menu = $state<{ x: number; y: number; t: Track } | null>(null);
  function openMenu(e: MouseEvent, t: Track) {
    e.preventDefault();
    e.stopPropagation();
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const x = e.type === 'contextmenu' ? e.clientX : r.right;
    const y = e.type === 'contextmenu' ? e.clientY : r.bottom + 4;
    menu = { x: Math.min(x, innerWidth - 200), y: Math.min(y, innerHeight - 200), t };
  }
</script>

<svelte:window onclick={() => (menu = null)} onkeydown={(e) => e.key === 'Escape' && (menu = null)} onblur={() => (menu = null)} />

<div class="list" bind:this={host} style:height="{tracks.length * ROW}px" role="list">
  <div class="window" style:transform="translateY({start * ROW}px)">
    {#each visible as t, i (t.id)}
      {@const idx = start + i}
      {@const active = t.id === currentId}
      <div
        class="row"
        class:active
        class:noalbum={!showAlbum}
        class:off={t.availability === 'none'}
        class:sel={selectable && selected.has(t.id)}
        class:enter={animate && idx < 14}
        style:--d="{Math.min(idx, 14) * 18}ms"
        role="listitem"
        ondblclick={() => onplay(idx)}
        oncontextmenu={(e) => openMenu(e, t)}
      >
        {#if selectable}
          <button class="check" class:on={selected.has(t.id)} onclick={() => toggle(t.id)} aria-label="选择">
            <Icon name="check" stroke={2.4} />
          </button>
        {/if}
        <button class="lead" onclick={() => onplay(idx)} aria-label="播放 {t.name}">
          {#if showCover}
            <span class="cover">
              <img use:fade src={img(t.cover, 40)} alt="" decoding="async" width="40" height="40" />
            </span>
          {:else}
            <span class="num tabular">{numbered ? idx + 1 : ''}</span>
          {/if}
          <span class="play-overlay" class:cover-mode={showCover}>
            {#if active && player.playing}
              <span class="bars" aria-hidden="true"><i></i><i></i><i></i></span>
            {:else}
              <Icon name="play" />
            {/if}
          </span>
        </button>
        <div class="title">
          <div class="name ellipsis">
            {t.name}{#if t.alias}<span class="alias">（{t.alias}）</span>{/if}
          </div>
          <div class="sub ellipsis">
            {#if t.availability === 'none'}<span class="badge">无版权</span>
            {:else if t.availability === 'paid'}<span class="badge vip">付费</span>
            {:else if t.fee === 1}<span class="badge vip">VIP</span>{/if}
            {artists(t)}
          </div>
        </div>
        {#if showAlbum}
          <div class="album ellipsis">{t.album}</div>
        {/if}
        <div class="dur tabular">{duration(t.duration)}</div>
        <div class="actions">
          <button
            class="icon-btn"
            onclick={() => app.download([t], t.name, { subtitle: artists(t), cover: t.cover })}
            aria-label="下载"
            title="下载"><Icon name="download" /></button
          >
          <button class="icon-btn" onclick={(e) => openMenu(e, t)} aria-label="更多" title="更多"><Icon name="more" /></button>
        </div>
      </div>
    {/each}
  </div>
</div>

{#if menu}
  {@const t = menu.t}
  <div class="menu" style:left="{menu.x}px" style:top="{menu.y}px" role="menu">
    <button role="menuitem" onclick={() => player.play(t)}><Icon name="play" />立即播放</button>
    <button role="menuitem" onclick={() => player.playNext(t)}><Icon name="queue" />下一首播放</button>
    <button role="menuitem" onclick={() => player.enqueue([t])}><Icon name="plus" />添加到播放列表</button>
    <hr />
    <button role="menuitem" onclick={() => app.download([t], t.name, { subtitle: artists(t), cover: t.cover })}
      ><Icon name="download" />下载</button
    >
    <button role="menuitem" onclick={() => app.openSong(t.id)}><Icon name="note" />歌曲详情</button>
    <button role="menuitem" onclick={() => app.openCollection('album', t.albumId)}><Icon name="disc" />前往专辑</button>
    <button role="menuitem" onclick={() => navigator.clipboard?.writeText(`https://music.163.com/song?id=${t.id}`)}
      ><Icon name="link" />复制链接</button
    >
  </div>
{/if}

<style>
  .list {
    position: relative;
    contain: strict;
    width: 100%;
  }
  .window {
    position: absolute;
    inset: 0 0 auto 0;
    will-change: transform;
  }
  .row {
    height: 56px;
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) minmax(0, 0.7fr) 52px 72px;
    align-items: center;
    gap: 12px;
    padding: 0 8px 0 8px;
    border-radius: var(--r);
    position: relative;
    transition: background-color var(--t-fast) var(--ease);
  }
  .row:has(.check) {
    grid-template-columns: 28px auto minmax(0, 1fr) minmax(0, 0.7fr) 52px 72px;
  }
  .row.noalbum {
    grid-template-columns: auto minmax(0, 1fr) 52px 72px;
  }
  .row.noalbum:has(.check) {
    grid-template-columns: 28px auto minmax(0, 1fr) 52px 72px;
  }
  .row::after {
    content: '';
    position: absolute;
    left: 60px;
    right: 8px;
    bottom: 0;
    height: 0.5px;
    background: var(--line);
  }
  .row:hover {
    background: var(--hover);
  }
  .row:hover::after,
  .row.active::after {
    opacity: 0;
  }
  .row.active {
    background: var(--bg-2);
  }
  .row.active .name {
    color: var(--accent);
  }
  .row.off .title,
  .row.off .album,
  .row.off .dur,
  .row.off .cover {
    opacity: 0.38;
  }
  .row.enter {
    animation: rise var(--t-slow) var(--ease-out) backwards;
    animation-delay: var(--d);
  }

  .lead {
    position: relative;
    width: 40px;
    height: 40px;
    display: grid;
    place-items: center;
    flex: none;
  }
  .cover {
    width: 40px;
    height: 40px;
    border-radius: var(--r-sm);
    overflow: hidden;
    background: var(--bg-3);
  }
  .cover img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .num {
    color: var(--fg-3);
    font-size: 13px;
    transition: opacity var(--t-fast) var(--ease);
  }
  .play-overlay {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    color: var(--fg);
    opacity: 0;
    border-radius: var(--r-sm);
    transition: opacity var(--t-fast) var(--ease);
  }
  .play-overlay.cover-mode {
    background: rgba(0, 0, 0, 0.42);
    color: #fff;
  }
  .play-overlay :global(svg) {
    width: 16px;
    height: 16px;
  }
  .row:hover .play-overlay,
  .row.active .play-overlay {
    opacity: 1;
  }
  .row:hover .num,
  .row.active .num {
    opacity: 0;
  }
  .row.active .play-overlay:not(.cover-mode) {
    color: var(--accent);
  }

  .bars {
    display: flex;
    gap: 2px;
    align-items: flex-end;
    height: 14px;
  }
  .bars i {
    width: 3px;
    height: 100%;
    border-radius: 1px;
    background: currentColor;
    transform-origin: bottom;
    animation: bar 0.9s var(--ease-in-out) infinite alternate;
  }
  .bars i:nth-child(2) {
    animation-delay: -0.3s;
  }
  .bars i:nth-child(3) {
    animation-delay: -0.6s;
  }
  @keyframes bar {
    from {
      transform: scaleY(0.25);
    }
    to {
      transform: scaleY(1);
    }
  }

  .title {
    min-width: 0;
  }
  .name {
    font-size: 14px;
    font-weight: 500;
    letter-spacing: -0.01em;
  }
  .alias {
    color: var(--fg-3);
    font-weight: 400;
  }
  .sub {
    font-size: 12px;
    color: var(--fg-2);
    display: flex;
    align-items: center;
    gap: 6px;
    margin-top: 1px;
  }
  .album {
    font-size: 13px;
    color: var(--fg-2);
  }
  .dur {
    font-size: 13px;
    color: var(--fg-3);
    text-align: right;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 2px;
    opacity: 0;
    transition: opacity var(--t-fast) var(--ease);
  }
  .row:hover .actions,
  .row:focus-within .actions {
    opacity: 1;
  }

  .check {
    width: 20px;
    height: 20px;
    border-radius: 50%;
    border: 1.5px solid var(--line-2);
    display: grid;
    place-items: center;
    color: transparent;
    transition:
      background-color var(--t-fast) var(--ease),
      border-color var(--t-fast) var(--ease),
      transform var(--t-fast) var(--ease);
  }
  .check :global(svg) {
    width: 12px;
    height: 12px;
  }
  .check.on {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
  }
  .check:active {
    transform: scale(0.88);
  }

  .menu {
    position: fixed;
    z-index: 60;
    min-width: 188px;
    padding: 5px;
    border-radius: var(--r);
    background: var(--bg-elev);
    box-shadow: var(--shadow);
    animation: menu-in var(--t) var(--ease) both;
    transform-origin: top left;
  }
  @keyframes menu-in {
    from {
      opacity: 0;
      transform: scale(0.96);
    }
  }
  .menu button {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 10px;
    height: 30px;
    padding: 0 10px;
    border-radius: var(--r-sm);
    font-size: 13px;
    text-align: left;
  }
  .menu button:hover {
    background: var(--accent);
    color: #fff;
  }
  .menu :global(svg) {
    width: 15px;
    height: 15px;
    opacity: 0.8;
  }
  .menu hr {
    border: 0;
    height: 0.5px;
    background: var(--line);
    margin: 4px 8px;
  }

  @media (max-width: 760px) {
    .row {
      grid-template-columns: auto minmax(0, 1fr) 44px 64px;
    }
    .row:has(.check) {
      grid-template-columns: 28px auto minmax(0, 1fr) 44px 64px;
    }
    .album {
      display: none;
    }
    .actions {
      opacity: 1;
    }
  }
</style>
