<script lang="ts">
  import { fade } from '../lib/fade';
  import Icon from './Icon.svelte';
  import { app, SOURCE_TABS } from '../lib/app.svelte';
  import { player } from '../lib/player.svelte';
  import { history, clearHistory } from '../lib/history.svelte';
  import { recent } from '../lib/recent.svelte';
  import { artists, collectionKindLabel, img, sourceShort } from '../lib/format';
  import type { PlaylistBrief, Track } from '../lib/types';

  const opened = $derived(history.items.filter((h) => h.kind !== 'query').slice(0, 6));
  const played = $derived(recent.items.slice(0, 6));
  const currentKey = $derived(player.current ? `${player.current.source}:${player.current.id}` : '');
  /** 加载完成后的内容；加载中为 null，模板据此渲染骨架 */
  const feed = $derived(app.homeLoading ? null : app.feed);

  const kindLabel = { song: '歌曲', playlist: '歌单', album: '专辑', query: '' } as const;

  function playNew(i: number) {
    const f = feed;
    if (f?.newSongs.length) player.playList(f.newSongs, i);
  }
</script>

<div class="home">
  <header class="page-head">
    <h1>主页</h1>
    <div class="seg-ctl" role="tablist" aria-label="音源">
      {#each SOURCE_TABS as [id, label] (id)}
        <button class:on={app.source === id} onclick={() => app.setSource(id)} role="tab" aria-selected={app.source === id}>{label}</button>
      {/each}
    </div>
  </header>

  {#if played.length}
    <section>
      <div class="sec-head">
        <h2>最近播放</h2>
        <button class="btn plain" onclick={() => player.playList(played, 0)}>播放全部</button>
      </div>
      <div class="songs">
        {#each played as t, i (`${t.source}:${t.id}`)}
          {@render songCell(t, i, played)}
        {/each}
      </div>
    </section>
  {/if}

  {#if opened.length}
    <section>
      <div class="sec-head">
        <h2>最近打开</h2>
        <button class="btn plain" onclick={clearHistory}>清除</button>
      </div>
      <div class="shelf">
        {#each opened as h, i (h.at)}
          <button class="card" style:--d="{i * 24}ms" onclick={() => app.submit(h.q)}>
            <span class="art">
              {#if h.cover}<img use:fade src={img(h.cover, 200)} alt="" decoding="async" />{/if}
            </span>
            <span class="card-title ellipsis">{h.title}</span>
            <span class="card-sub ellipsis">{kindLabel[h.kind]}</span>
          </button>
        {/each}
      </div>
    </section>
  {/if}

  <section>
    <div class="sec-head"><h2>推荐歌单</h2></div>
    <div class="shelf">
      {#if feed}
        {#each feed.playlists.slice(0, 5) as p, i (`${p.source}:${p.id}`)}
          {@render playlistCard(p, i)}
        {/each}
      {:else}
        {#each { length: 5 } as _}
          <div class="card skel"><span class="art"></span><span class="sk-line"></span><span class="sk-line sm"></span></div>
        {/each}
      {/if}
    </div>
  </section>

  {#if !feed}
    <section>
      <div class="sec-head"><h2>最新音乐</h2></div>
      <div class="songs">
        {#each { length: 9 } as _}
          <div class="song skel"><span class="s-art"></span><span class="s-txt"><span class="sk-line"></span><span class="sk-line sm"></span></span></div>
        {/each}
      </div>
    </section>
  {:else}
    {#if feed.newSongs.length}
      <section>
        <div class="sec-head">
          <h2>最新音乐</h2>
          <button class="btn plain" onclick={() => playNew(0)}>播放全部</button>
        </div>
        <div class="songs">
          {#each feed.newSongs.slice(0, 9) as t, i (`${t.source}:${t.id}`)}
            {@render songCell(t, i, feed.newSongs)}
          {/each}
        </div>
      </section>
    {/if}
    {#if feed.charts.length}
      <section>
        <div class="sec-head"><h2>排行榜</h2></div>
        <div class="shelf">
          {#each feed.charts.slice(0, 5) as c, i (`${c.source}:${c.id}`)}
            {@render playlistCard(c, i)}
          {/each}
        </div>
      </section>
    {/if}
  {/if}
</div>

{#snippet playlistCard(p: PlaylistBrief, i: number)}
  <button class="card" style:--d="{Math.min(i, 8) * 24}ms" onclick={() => app.openCollection(p.kind || 'playlist', String(p.id), undefined, p.source)}>
    <span class="art">
      {#if p.cover}<img use:fade src={img(p.cover, 220)} alt="" decoding="async" />{/if}
      {#if app.source === 'all'}<span class="src-tag">{sourceShort(p.source)}</span>{/if}
      <span class="play-badge" aria-hidden="true"><Icon name="play" /></span>
    </span>
    <span class="card-title clamp2">{p.name}</span>
    {#if p.subtitle}<span class="card-sub ellipsis">{p.subtitle}</span>{/if}
  </button>
{/snippet}

{#snippet songCell(t: Track, i: number, list: Track[] = [])}
  {@const active = `${t.source}:${t.id}` === currentKey}
  <div class="song" class:active class:off={t.availability === 'none'}>
    <button class="s-main" onclick={() => (list.length ? player.playList(list, i) : playNew(i))}>
      <span class="s-art">
        <img use:fade src={img(t.cover, 56)} alt="" decoding="async" width="48" height="48" />
        <span class="s-over"><Icon name={active && player.playing ? 'pause' : 'play'} /></span>
      </span>
      <span class="s-txt">
        <span class="s-name ellipsis">{t.name}</span>
        <span class="s-sub ellipsis">
          {#if app.source === 'all'}<span class="badge">{sourceShort(t.source)}</span>{/if}
          {#if t.fee === 1}<span class="badge vip">VIP</span>{/if}
          {artists(t)}
        </span>
      </span>
    </button>
    <button class="icon-btn dl" onclick={() => app.download([t], t.name, { subtitle: artists(t), cover: t.cover })} title="下载" aria-label="下载">
      <Icon name="download" />
    </button>
  </div>
{/snippet}

<style>
  .home {
    padding: 4px 4px 32px;
  }
  .page-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 6px 0 16px;
  }
  h1 {
    margin: 0;
    font: 700 30px/1.1 var(--font-display);
    letter-spacing: -0.028em;
  }
  .src-tag {
    position: absolute;
    left: 8px;
    top: 8px;
    padding: 2px 6px;
    border-radius: 5px;
    background: rgba(0, 0, 0, 0.52);
    color: #fff;
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.02em;
  }
  section {
    margin-top: 32px;
  }
  .sec-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    margin-bottom: 14px;
  }
  h2 {
    margin: 0;
    font: 700 20px/1.2 var(--font-display);
    letter-spacing: -0.02em;
  }

  /* 歌单与榜单网格 */
  .shelf {
    display: grid;
    grid-template-columns: repeat(5, minmax(0, 1fr));
    gap: 24px 18px;
  }

  .card {
    display: flex;
    flex-direction: column;
    text-align: left;
    min-width: 0;
    animation: rise var(--t-slow) var(--ease-out) backwards;
    animation-delay: var(--d, 0ms);
  }
  .art {
    position: relative;
    width: 100%;
    aspect-ratio: 1;
    border-radius: var(--r);
    overflow: hidden;
    background: var(--bg-2);
    box-shadow: 0 2px 10px rgba(0, 0, 0, 0.06), inset 0 0 0 0.5px var(--line);
    transition: transform var(--t) var(--ease), box-shadow var(--t) var(--ease);
  }
  .card:hover .art {
    transform: translateY(-2px);
    box-shadow: var(--cover-shadow);
  }
  .card:active .art {
    transform: scale(0.98);
  }
  .art img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .play-badge {
    position: absolute;
    right: 10px;
    bottom: 10px;
    width: 34px;
    height: 34px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    background: rgba(255, 255, 255, 0.92);
    color: #1d1d1f;
    box-shadow: 0 4px 14px rgba(0, 0, 0, 0.22);
    opacity: 0;
    transform: scale(0.88);
    transition: opacity var(--t-fast) var(--ease), transform var(--t) var(--ease-spring);
  }
  .play-badge :global(svg) {
    width: 15px;
    height: 15px;
    margin-left: 1px;
  }
  .card:hover .play-badge {
    opacity: 1;
    transform: scale(1);
  }
  .card-title {
    margin-top: 8px;
    font-size: 13px;
    font-weight: 500;
    line-height: 1.35;
    letter-spacing: -0.008em;
  }
  .clamp2 {
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .card-sub {
    margin-top: 2px;
    font-size: 12px;
    color: var(--fg-2);
  }

  /* 最新音乐：三列紧凑行 */
  .songs {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 2px 24px;
  }
  .song {
    position: relative;
    display: flex;
    align-items: center;
    border-radius: var(--r);
    padding-right: 4px;
    transition: background-color var(--t-fast) var(--ease);
  }
  .song::after {
    content: '';
    position: absolute;
    left: 66px;
    right: 8px;
    bottom: 0;
    height: 0.5px;
    background: var(--line);
  }
  .song:hover {
    background: var(--hover);
  }
  .song:hover::after {
    opacity: 0;
  }
  .song.active .s-name {
    color: var(--accent);
  }
  .song.off {
    opacity: 0.4;
  }
  .s-main {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 12px;
    height: 60px;
    padding: 0 6px;
    text-align: left;
  }
  .s-art {
    position: relative;
    width: 48px;
    height: 48px;
    border-radius: var(--r-sm);
    overflow: hidden;
    background: var(--bg-2);
    flex: none;
    box-shadow: inset 0 0 0 0.5px var(--line);
  }
  .s-art img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .s-over {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    background: rgba(0, 0, 0, 0.42);
    color: #fff;
    opacity: 0;
    transition: opacity var(--t-fast) var(--ease);
  }
  .s-over :global(svg) {
    width: 16px;
    height: 16px;
  }
  .song:hover .s-over,
  .song.active .s-over {
    opacity: 1;
  }
  .s-txt {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .s-name {
    font-size: 14px;
    font-weight: 500;
    letter-spacing: -0.01em;
  }
  .s-sub {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 12px;
    color: var(--fg-2);
  }
  .dl {
    opacity: 0;
    transition: opacity var(--t-fast) var(--ease);
  }
  .song:hover .dl {
    opacity: 1;
  }
  .dl :global(svg) {
    width: 16px;
    height: 16px;
  }

  /* 骨架占位 */
  .skel {
    pointer-events: none;
  }
  .sk-line {
    display: block;
    height: 12px;
    width: 78%;
    margin-top: 10px;
    border-radius: 4px;
    background: var(--bg-2);
  }
  .sk-line.sm {
    width: 48%;
    height: 10px;
    margin-top: 6px;
  }
  .song.skel {
    height: 60px;
    gap: 12px;
    padding: 0 6px;
  }
  .song.skel .sk-line {
    margin-top: 0;
  }

  @media (max-width: 960px) and (min-width: 641px) {
    .shelf {
      grid-template-columns: repeat(4, minmax(0, 1fr));
    }
    .songs {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
  @media (max-width: 640px) {
    .page-head {
      flex-direction: column;
      align-items: stretch;
      gap: 12px;
      padding: 2px 0 12px;
    }
    h1 {
      font-size: 24px;
    }
    .page-head .seg-ctl {
      display: grid;
      grid-template-columns: repeat(4, 1fr);
    }
    .page-head .seg-ctl button {
      padding: 0 4px;
    }
    section {
      margin-top: 26px;
    }
    .shelf {
      grid-template-columns: repeat(2, minmax(0, 1fr));
      gap: 16px 12px;
    }
    .songs {
      grid-template-columns: 1fr;
      gap: 0;
    }
    .play-badge {
      display: none;
    }
    .song {
      padding-right: 0;
    }
    .dl {
      opacity: 1;
    }
    .s-main {
      height: 58px;
    }
    .card:active .art {
      transform: scale(0.96);
    }
  }
</style>
