<script lang="ts">
  import Icon from './Icon.svelte';
  import TrackList from './TrackList.svelte';
  import { app } from '../lib/app.svelte';
  import { player } from '../lib/player.svelte';
  import { library } from '../lib/library.svelte';
  import { toast } from '../lib/toast.svelte';
  import { fade } from '../lib/fade';
  import { artists, img } from '../lib/format';
  import type { LibTrack, Track } from '../lib/types';

  let { scroller }: { scroller: HTMLElement | undefined } = $props();

  type Tab = 'likes' | 'playlists' | 'plays';
  let tab = $state<Tab>('likes');

  /** 资料库曲目 → Track（播放时解析器会重新判定可用性） */
  function toTrack(t: LibTrack): Track {
    return {
      id: t.id,
      source: t.source,
      name: t.name,
      alias: null,
      artists: t.artists,
      albumId: t.albumId,
      album: t.album,
      cover: t.cover,
      duration: t.duration,
      fee: 0,
      availability: 'ok',
      isrc: t.isrc ?? null,
      alts: t.alts.map((k) => {
        const [source, ...rest] = k.split(':');
        return { source, id: rest.join(':') };
      }),
    };
  }

  const liked = $derived(library.likes.map(toTrack));
  const played = $derived(library.plays.map((p) => toTrack(p as unknown as LibTrack)));

  async function newPlaylist() {
    const name = prompt('新歌单名称');
    if (!name?.trim()) return;
    const pl = await library.createPlaylist(name);
    if (pl) {
      toast.success(`已创建「${pl.name}」`);
      app.openPlaylist(pl);
    }
  }

  async function clearPlays() {
    await library.clearPlays();
    toast.info('已清空播放记录');
  }
</script>

<header class="head">
  <div class="icon"><Icon name="library" size={26} /></div>
  <div class="info">
    <div class="eyebrow">你的音乐</div>
    <h1>资料库</h1>
    <div class="meta muted">
      {library.likes.length} 首喜欢 · {library.playlists.length} 个歌单 · {library.plays.length} 条播放记录
    </div>
  </div>
</header>

<div class="seg-ctl tabs" role="tablist" aria-label="资料库分区">
  <button class:on={tab === 'likes'} onclick={() => (tab = 'likes')} role="tab" aria-selected={tab === 'likes'}>喜欢的歌曲</button>
  <button class:on={tab === 'playlists'} onclick={() => (tab = 'playlists')} role="tab" aria-selected={tab === 'playlists'}>歌单</button>
  <button class:on={tab === 'plays'} onclick={() => (tab = 'plays')} role="tab" aria-selected={tab === 'plays'}>播放记录</button>
</div>

{#if tab === 'likes'}
  {#if !liked.length}
    <div class="blank">
      <Icon name="heart" size={28} />
      <div class="blank-title">还没有喜欢的歌曲</div>
      <div class="blank-sub">在歌曲上点 ♥ 或右键菜单「喜欢」，就会出现在这里</div>
    </div>
  {:else}
    <div class="bar">
      <button class="btn primary" onclick={() => player.playList(liked, 0)}><Icon name="play" />播放</button>
      <button class="btn" onclick={() => (player.shuffle = true, player.playList(liked, Math.floor(Math.random() * liked.length)))}><Icon name="shuffle" />随机播放</button>
      <span class="muted count tabular">{liked.length} 首</span>
    </div>
    <TrackList tracks={liked} {scroller} onplay={(i) => player.playList(liked, i)} />
  {/if}
{:else if tab === 'playlists'}
  <div class="bar">
    <button class="btn primary" onclick={newPlaylist}><Icon name="plus" />新建歌单</button>
  </div>
  {#if !library.playlists.length}
    <div class="blank">
      <Icon name="folderPlus" size={28} />
      <div class="blank-title">还没有歌单</div>
      <div class="blank-sub">新建一个歌单，把喜欢的歌整理起来</div>
    </div>
  {:else}
    <div class="grid">
      {#each library.playlists as pl (pl.id)}
        <button class="card" onclick={() => app.openPlaylist(pl)}>
          <span class="art">
            {#if pl.cover}
              <img use:fade src={img(pl.cover, 220)} alt="" width="220" height="220" loading="lazy" decoding="async" />
            {:else}
              <span class="ph"><Icon name="note" size={28} /></span>
            {/if}
          </span>
          <span class="c-name ellipsis">{pl.name}</span>
          <span class="c-sub muted">{pl.tracks.length} 首</span>
        </button>
      {/each}
    </div>
  {/if}
{:else}
  <div class="bar">
    <button class="btn primary" onclick={() => player.playList(played, 0)} disabled={!played.length}><Icon name="play" />播放</button>
    <button class="btn" onclick={clearPlays} disabled={!played.length}><Icon name="trash" />清空</button>
    <span class="muted count tabular">{played.length} 条</span>
  </div>
  {#if !played.length}
    <div class="blank">
      <Icon name="clock" size={28} />
      <div class="blank-title">还没有播放记录</div>
      <div class="blank-sub">听过的歌会自动记录在这里</div>
    </div>
  {:else}
    <TrackList tracks={played} {scroller} onplay={(i) => player.playList(played, i)} />
  {/if}
{/if}

<style>
  .head {
    display: flex;
    gap: 24px;
    align-items: flex-end;
    padding: 8px 8px 20px;
    animation: rise var(--t-slow) var(--ease-out) backwards;
  }
  .icon {
    width: 160px;
    height: 160px;
    flex: none;
    display: grid;
    place-items: center;
    border-radius: var(--r-lg);
    overflow: hidden;
    background: linear-gradient(150deg, var(--accent-soft), var(--bg-2));
    box-shadow: var(--cover-shadow);
    color: var(--accent);
  }
  .info {
    min-width: 0;
    flex: 1;
  }
  .eyebrow {
    font-size: var(--fs-caption);
    font-weight: 600;
    color: var(--fg-2);
    letter-spacing: 0.02em;
    margin-bottom: 6px;
  }
  h1 {
    margin: 0;
    font: 700 var(--fs-title1)/1.15 var(--font-display);
    letter-spacing: -0.025em;
  }
  .meta {
    margin-top: 6px;
    font-size: var(--fs-footnote);
  }
  .tabs {
    margin: 0 8px 16px;
  }
  .bar {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 8px 12px;
  }
  .count {
    margin-left: auto;
    font-size: var(--fs-footnote);
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
    gap: 18px;
    padding: 4px 8px 36px;
  }
  .card {
    display: flex;
    flex-direction: column;
    gap: 4px;
    text-align: left;
    border-radius: var(--r-lg);
    transition: transform var(--t-fast) var(--ease);
  }
  .card:active {
    transform: scale(0.98);
  }
  .art {
    position: relative;
    aspect-ratio: 1;
    border-radius: var(--r-lg);
    overflow: hidden;
    background: var(--bg-3);
    box-shadow: var(--cover-shadow);
    margin-bottom: 6px;
  }
  .art img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .ph {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    color: var(--fg-3);
  }
  .c-name {
    font-size: var(--fs-subhead);
    font-weight: 600;
  }
  .c-sub {
    font-size: var(--fs-caption2);
  }
  .blank {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    padding: 64px 20px;
    text-align: center;
    color: var(--fg-3);
    animation: rise var(--t-slow) var(--ease-out) backwards;
  }
  .blank :global(svg) {
    width: 32px;
    height: 32px;
    margin-bottom: 4px;
    opacity: 0.7;
  }
  .blank-title {
    font-size: var(--fs-headline);
    font-weight: 600;
    color: var(--fg);
  }
  .blank-sub {
    font-size: var(--fs-footnote);
    color: var(--fg-2);
    max-width: 340px;
  }

  @media (max-width: 760px) {
    .head {
      flex-direction: column;
      align-items: center;
      text-align: center;
      gap: 14px;
      padding: 8px 0 16px;
    }
    .icon {
      width: min(40vw, 140px);
      height: min(40vw, 140px);
      border-radius: var(--r-xl);
    }
    h1 {
      font-size: var(--fs-title2);
    }
    .tabs {
      display: grid;
      grid-template-columns: repeat(3, 1fr);
      margin: 0 0 14px;
    }
  }
</style>
