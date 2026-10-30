<script lang="ts">
  import { fade } from '../lib/fade';
  import Icon from './Icon.svelte';
  import TrackList from './TrackList.svelte';
  import { app } from '../lib/app.svelte';
  import { player } from '../lib/player.svelte';
  import { toast } from '../lib/toast.svelte';
  import { img } from '../lib/format';
  import type { Track } from '../lib/types';

  let {
    kind,
    title,
    eyebrow,
    owner = '',
    meta = '',
    cover = '',
    description = '',
    tracks,
    scroller,
    isAlbum = false,
    onreachend,
    footer,
  }: {
    kind: 'collection' | 'search';
    title: string;
    eyebrow: string;
    owner?: string;
    meta?: string;
    cover?: string;
    description?: string;
    tracks: Track[];
    scroller: HTMLElement | undefined;
    isAlbum?: boolean;
    onreachend?: () => void;
    footer?: import('svelte').Snippet;
  } = $props();

  let selecting = $state(false);
  let selected = $state(new Set<string>());
  let filter = $state('');
  let descOpen = $state(false);

  const shown = $derived.by(() => {
    const f = filter.trim().toLowerCase();
    if (!f) return tracks;
    return tracks.filter(
      (t) =>
        t.name.toLowerCase().includes(f) ||
        t.album.toLowerCase().includes(f) ||
        t.artists.some((a) => a.name.toLowerCase().includes(f)),
    );
  });

  const total = $derived(tracks.reduce((s, t) => s + t.duration, 0));
  const picked = $derived(selecting ? tracks.filter((t) => selected.has(t.id)) : tracks);

  function startSelect() {
    selecting = true;
    selected = new Set(tracks.map((t) => t.id));
  }

  function toggleAll() {
    selected = selected.size === shown.length ? new Set() : new Set(shown.map((t) => t.id));
  }

  function playAll(shuffle = false) {
    if (!picked.length) return;
    player.shuffle = shuffle;
    player.playList(picked, shuffle ? Math.floor(Math.random() * picked.length) : 0);
  }

  function enqueueAll() {
    const n = player.enqueue(picked);
    toast.info(n ? `已添加 ${n} 首到播放列表` : '这些歌曲已在播放列表中');
  }

  function download() {
    app.download(picked, title, { subtitle: owner, cover, collection: kind === 'collection' });
    selecting = false;
  }

  function hours(ms: number) {
    const m = Math.round(ms / 60000);
    return m >= 60 ? `${Math.floor(m / 60)} 小时 ${m % 60} 分钟` : `${m} 分钟`;
  }
</script>

<header class="head" class:noart={!cover}>
  {#if cover}
    <div class="art">
      <img use:fade src={img(cover, 220)} alt="" width="220" height="220" decoding="async" />
    </div>
  {/if}
  <div class="info">
    <div class="eyebrow">{eyebrow}</div>
    <h1>{title}</h1>
    {#if owner}<div class="owner">{owner}</div>{/if}
    <div class="meta muted">
      {#if meta}{meta} · {/if}{tracks.length} 首{#if total} · {hours(total)}{/if}
    </div>
    {#if description}
      <button class="desc" class:open={descOpen} onclick={() => (descOpen = !descOpen)}>
        <p>{description}</p>
      </button>
    {/if}
    <div class="cta">
      <button class="btn primary lg" onclick={() => playAll()} disabled={!picked.length}>
        <Icon name="play" />播放
      </button>
      <button class="btn lg" onclick={() => playAll(true)} disabled={!picked.length}>
        <Icon name="shuffle" />随机播放
      </button>
      <button class="btn lg" onclick={download} disabled={!picked.length}>
        <Icon name="download" />{selecting ? `下载 ${picked.length} 首` : '下载全部'}
      </button>
      <button class="icon-btn" onclick={enqueueAll} title="添加到播放列表" aria-label="添加到播放列表">
        <Icon name="plus" />
      </button>
    </div>
  </div>
</header>

<div class="toolbar">
  {#if selecting}
    <button class="btn plain" onclick={toggleAll}>{selected.size === shown.length ? '取消全选' : '全选'}</button>
    <span class="muted count tabular">已选 {selected.size} 首</span>
    <button class="btn plain done" onclick={() => (selecting = false)}>完成</button>
  {:else}
    <button class="btn plain" onclick={startSelect}>选择</button>
    {#if tracks.length > 12}
      <div class="filter">
        <Icon name="search" />
        <input bind:value={filter} placeholder="在列表中筛选" spellcheck="false" />
      </div>
    {/if}
  {/if}
</div>

<TrackList
  tracks={shown}
  {scroller}
  selectable={selecting}
  bind:selected
  showCover={!isAlbum}
  numbered={isAlbum}
  showAlbum={!isAlbum}
  onplay={(i) => player.playList(shown, i)}
  {onreachend}
/>

{#if footer}{@render footer()}{/if}

<style>
  .head {
    display: flex;
    gap: 32px;
    align-items: flex-end;
    padding: 8px 8px 28px;
    animation: rise var(--t-slow) var(--ease-out) backwards;
  }
  .head.noart {
    padding-top: 16px;
  }
  .art {
    width: 220px;
    height: 220px;
    flex: none;
    border-radius: var(--r-lg);
    overflow: hidden;
    background: var(--bg-3);
    box-shadow: var(--cover-shadow);
  }
  .art img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .info {
    min-width: 0;
    flex: 1;
  }
  .eyebrow {
    font-size: 12px;
    font-weight: 600;
    color: var(--fg-2);
    letter-spacing: 0.02em;
    margin-bottom: 6px;
  }
  h1 {
    margin: 0;
    font: 700 30px/1.15 var(--font-display);
    letter-spacing: -0.025em;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .owner {
    margin-top: 4px;
    font-size: 18px;
    font-weight: 500;
    color: var(--accent);
    letter-spacing: -0.015em;
  }
  .meta {
    margin-top: 6px;
    font-size: 13px;
  }
  .desc {
    display: block;
    margin-top: 10px;
    max-width: 640px;
    text-align: left;
    color: var(--fg-2);
    font-size: 13px;
  }
  .desc p {
    margin: 0;
    white-space: pre-line;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .desc.open p {
    -webkit-line-clamp: unset;
    line-clamp: unset;
  }
  .cta {
    display: flex;
    gap: 10px;
    align-items: center;
    margin-top: 18px;
    flex-wrap: wrap;
  }
  .toolbar {
    position: sticky;
    top: 0;
    z-index: 5;
    display: flex;
    align-items: center;
    gap: 8px;
    height: 44px;
    margin: 0 0 4px;
    padding: 0 4px;
    background: var(--bg);
    border-bottom: 0.5px solid var(--line);
  }
  .count {
    font-size: 13px;
  }
  .done {
    margin-left: auto;
    font-weight: 600;
  }
  .filter {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: 6px;
    height: 28px;
    width: 220px;
    padding: 0 10px;
    border-radius: 8px;
    background: var(--bg-2);
    color: var(--fg-3);
  }
  .filter :global(svg) {
    width: 14px;
    height: 14px;
    flex: none;
  }
  .filter input {
    flex: 1;
    min-width: 0;
    border: 0;
    outline: 0;
    background: transparent;
    font-size: 13px;
    color: var(--fg);
  }

  @media (max-width: 760px) {
    .head {
      flex-direction: column;
      align-items: flex-start;
      gap: 20px;
    }
    .art {
      width: 160px;
      height: 160px;
    }
    h1 {
      font-size: 24px;
    }
    .filter {
      width: 150px;
    }
  }
</style>
