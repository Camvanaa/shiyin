<script lang="ts">
  import { fade } from '../lib/fade';
  import Icon from './Icon.svelte';
  import SourceFilter from './SourceFilter.svelte';
  import { app } from '../lib/app.svelte';
  import { player } from '../lib/player.svelte';
  import { history, clearHistory } from '../lib/history.svelte';
  import { recent } from '../lib/recent.svelte';
  import { artists, collectionKindLabel, img, sourceShort } from '../lib/format';
  import type { PlaylistBrief, Track } from '../lib/types';

  const opened = $derived(history.items.filter((h) => h.kind !== 'query').slice(0, 6));
  const played = $derived(recent.items.slice(0, 12));
  const currentKey = $derived(player.current ? `${player.current.source}:${player.current.id}` : '');
  /** 加载完成后的内容；加载中为 null，模板据此渲染骨架 */
  const feed = $derived(app.homeLoading ? null : app.feed);

  const kindLabel = { song: '歌曲', playlist: '歌单', album: '专辑', query: '' } as const;

  /** 移动端「最新音乐」按 3 首一页横向分页 */
  const PAGE = 3;
  const songPages = $derived.by(() => {
    const list = feed?.newSongs.slice(0, 12) ?? [];
    const pages: Track[][] = [];
    for (let i = 0; i < list.length; i += PAGE) pages.push(list.slice(i, i + PAGE));
    return pages;
  });

  function playFrom(list: Track[], i: number) {
    if (list.length) player.playList(list, i);
  }
</script>

<div class="home">
  <header class="page-head">
    <h1>主页</h1>
    <SourceFilter />
  </header>

  {#if played.length}
    <section>
      <button class="sec-head link" onclick={() => app.openHistory()}>
        <h2>最近播放</h2>
        <span class="more-arrow"><Icon name="chevronRight" size={18} stroke={2.2} /></span>
      </button>
      <!-- 桌面：3 列紧凑行；移动端：横向滑动的小封面 -->
      <div class="songs desk">
        {#each played.slice(0, 6) as t, i (`${t.source}:${t.id}`)}
          {@render songCell(t, i, played)}
        {/each}
      </div>
      <div class="rail">
        {#each played as t, i (`${t.source}:${t.id}`)}
          {@const active = `${t.source}:${t.id}` === currentKey}
          <button class="tile" class:active onclick={() => playFrom(played, i)}>
            <span class="tile-art">
              <img use:fade src={img(t.cover, 120)} alt="" decoding="async" loading="lazy" />
              {#if active}<span class="tile-now"><Icon name={player.playing ? 'pause' : 'play'} size={16} /></span>{/if}
            </span>
            <span class="tile-name ellipsis">{t.name}</span>
            <span class="tile-sub ellipsis">{artists(t)}</span>
          </button>
        {/each}
      </div>
    </section>
  {/if}

  {#if opened.length}
    <section class="desk">
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

  {#if !app.homeLoading && !app.feed}
    <div class="blank">
      <Icon name="compass" size={28} />
      <div class="blank-title">内容加载失败</div>
      <div class="blank-sub">{app.homeError || '请检查网络后重试'}</div>
      <button class="btn primary" onclick={() => app.loadHome()} style="margin-top:8px">重试</button>
    </div>
  {:else}
  <section>
    <div class="sec-head"><h2>推荐歌单</h2></div>
    <div class="shelf">
      {#if feed}
        {#each feed.playlists.slice(0, 10) as p, i (`${p.source}:${p.id}`)}
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
          <button class="btn plain" onclick={() => playFrom(feed.newSongs, 0)}>播放全部</button>
        </div>
        <div class="songs desk">
          {#each feed.newSongs.slice(0, 9) as t, i (`${t.source}:${t.id}`)}
            {@render songCell(t, i, feed.newSongs)}
          {/each}
        </div>
        <!-- 移动端：每页 3 首，横向翻页 -->
        <div class="pages">
          {#each songPages as page, p (p)}
            <div class="pg">
              {#each page as t, i (`${t.source}:${t.id}`)}
                {@render songCell(t, p * PAGE + i, feed.newSongs)}
              {/each}
            </div>
          {/each}
        </div>
      </section>
    {/if}
    {#if feed.charts.length}
      <section>
        <div class="sec-head"><h2>排行榜</h2></div>
        <div class="shelf desk">
          {#each feed.charts.slice(0, 5) as c, i (`${c.source}:${c.id}`)}
            {@render playlistCard(c, i)}
          {/each}
        </div>
        <!-- 移动端：紧凑榜单行 -->
        <div class="charts">
          {#each feed.charts.slice(0, 6) as c, i (`${c.source}:${c.id}`)}
            <button class="chart" style:--d="{i * 30}ms" onclick={() => app.openCollection(c.kind || 'toplist', String(c.id), undefined, c.source)}>
              <span class="chart-rank tabular">{i + 1}</span>
              <span class="chart-art">
                {#if c.cover}<img use:fade src={img(c.cover, 112)} alt="" decoding="async" loading="lazy" />{/if}
              </span>
              <span class="chart-txt">
                <span class="chart-name ellipsis">{c.name}</span>
                <span class="chart-sub ellipsis">{sourceShort(c.source)}{#if c.subtitle} · {c.subtitle}{/if}</span>
              </span>
              <Icon name="chevronRight" size={16} />
            </button>
          {/each}
        </div>
      </section>
    {/if}
  {/if}
  {/if}
</div>

{#snippet playlistCard(p: PlaylistBrief, i: number)}
  <button class="card" style:--d="{Math.min(i, 8) * 24}ms" onclick={() => app.openCollection(p.kind || 'playlist', String(p.id), undefined, p.source)}>
    <span class="art">
      {#if p.cover}<img use:fade src={img(p.cover, 220)} alt="" decoding="async" loading="lazy" />{/if}
      {#if app.source === 'all'}<span class="src-tag">{sourceShort(p.source)}</span>{/if}
      <span class="play-badge" aria-hidden="true"><Icon name="play" /></span>
    </span>
    <span class="card-title clamp2">{p.name}</span>
    {#if p.subtitle}<span class="card-sub ellipsis">{p.subtitle}</span>{/if}
  </button>
{/snippet}

{#snippet songCell(t: Track, i: number, list: Track[])}
  {@const active = `${t.source}:${t.id}` === currentKey}
  <div class="song" class:active class:off={t.availability === 'none'}>
    <button class="s-main" onclick={() => playFrom(list, i)}>
      <span class="s-art">
        <img use:fade src={img(t.cover, 56)} alt="" decoding="async" width="48" height="48" loading="lazy" />
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
    font: 700 var(--fs-title1)/1.1 var(--font-display);
    letter-spacing: -0.028em;
  }
  .src-tag {
    position: absolute;
    left: 8px;
    top: 8px;
    padding: 2px 6px;
    border-radius: var(--r-sm);
    background: rgba(0, 0, 0, 0.52);
    color: var(--white);
    font-size: var(--fs-micro);
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
  .sec-head.link {
    width: 100%;
    text-align: left;
    align-items: center;
    justify-content: flex-start;
    gap: 2px;
    color: inherit;
  }
  .more-arrow {
    display: inline-grid;
    place-items: center;
    color: var(--fg-3);
    transition: transform var(--t) var(--ease), color var(--t-fast) var(--ease);
  }
  .sec-head.link:hover .more-arrow {
    transform: translateX(2px);
    color: var(--fg-2);
  }
  h2 {
    margin: 0;
    font: 700 var(--fs-title3)/1.2 var(--font-display);
    letter-spacing: -0.02em;
  }

  /* 歌单与榜单网格 */
  .shelf {
    display: grid;
    grid-template-columns: repeat(5, minmax(0, 1fr));
    gap: 24px 18px;
  }
  .shelf > :nth-child(n + 6) {
    display: none;
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
    background: linear-gradient(150deg, var(--bg-3), var(--bg-2));
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
    color: var(--ink);
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
    font-size: var(--fs-footnote);
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
    font-size: var(--fs-caption);
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
    background: linear-gradient(150deg, var(--bg-3), var(--bg-2));
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
    color: var(--white);
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
    font-size: var(--fs-subhead);
    font-weight: 500;
    letter-spacing: -0.01em;
  }
  .s-sub {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: var(--fs-caption);
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
    border-radius: var(--r-xs);
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

  /* 移动端专属元素在桌面不渲染；.desk 在移动端隐藏 */
  .rail,
  .pages,
  .charts {
    display: none;
  }

  @media (max-width: 960px) and (min-width: 641px) {
    .shelf {
      grid-template-columns: repeat(4, minmax(0, 1fr));
    }
    .shelf > :nth-child(n + 5) {
      display: none;
    }
    .songs {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }

  /* ---------- 移动端：横向滑动 + 分页列表 ---------- */
  @media (max-width: 640px) {
    .desk {
      display: none !important;
    }
    .home {
      padding: 0 0 24px;
    }
    .page-head {
      flex-direction: column;
      align-items: stretch;
      gap: 12px;
      padding: 2px 0 10px;
    }
    h1 {
      font-size: var(--fs-title1);
    }
    section {
      margin-top: 24px;
    }
    .sec-head {
      margin-bottom: 10px;
      padding: 0 2px;
    }
    h2 {
      font-size: var(--fs-title3);
    }

    /* 横向滚动容器：左右出血到屏幕边缘，snap 对齐 */
    .shelf,
    .rail,
    .pages {
      display: flex;
      gap: 12px;
      margin: 0 -12px;
      padding: 2px 12px 6px;
      overflow-x: auto;
      overscroll-behavior-x: contain;
      scroll-snap-type: x mandatory;
      scroll-padding-left: 12px;
      scrollbar-width: none;
      -webkit-overflow-scrolling: touch;
    }
    .shelf::-webkit-scrollbar,
    .rail::-webkit-scrollbar,
    .pages::-webkit-scrollbar {
      display: none;
    }
    .shelf > :nth-child(n + 5),
    .shelf > :nth-child(n + 6) {
      display: flex;
    }
    .shelf .card {
      flex: none;
      width: 148px;
      scroll-snap-align: start;
      animation: none;
    }
    .art {
      border-radius: var(--r);
    }
    .play-badge {
      display: none;
    }
    .card-title {
      font-size: var(--fs-footnote);
    }

    /* 最近播放：小瓦片 */
    .rail .tile {
      flex: none;
      width: 104px;
      display: flex;
      flex-direction: column;
      text-align: left;
      scroll-snap-align: start;
    }
    .tile-art {
      position: relative;
      width: 104px;
      height: 104px;
      border-radius: var(--r);
      overflow: hidden;
      background: linear-gradient(150deg, var(--bg-3), var(--bg-2));
      box-shadow: inset 0 0 0 0.5px var(--line);
      transition: transform var(--t-fast) var(--ease);
    }
    .tile:active .tile-art {
      transform: scale(0.96);
    }
    .tile-art img {
      width: 100%;
      height: 100%;
      object-fit: cover;
    }
    .tile-now {
      position: absolute;
      inset: 0;
      display: grid;
      place-items: center;
      background: rgba(0, 0, 0, 0.42);
      color: var(--white);
    }
    .tile-name {
      margin-top: 6px;
      font-size: var(--fs-caption);
      font-weight: 500;
      letter-spacing: -0.005em;
    }
    .tile.active .tile-name {
      color: var(--accent);
    }
    .tile-sub {
      font-size: var(--fs-caption2);
      color: var(--fg-2);
    }

    /* 最新音乐：每页 3 首，页宽 = 视口减去右侧露出的下一页边缘 */
    .pages .pg {
      flex: none;
      width: calc(100vw - 12px - 32px);
      scroll-snap-align: start;
    }
    .song {
      padding-right: 0;
    }
    .song::after {
      left: 60px;
      right: 0;
    }
    .s-main {
      height: 56px;
      gap: 12px;
      padding: 0 4px 0 2px;
    }
    .dl {
      opacity: 1;
      width: 40px;
      height: 40px;
      color: var(--fg-3);
    }
    .s-name {
      font-size: var(--fs-callout);
    }

    /* 排行榜：紧凑榜单行 */
    .charts {
      display: flex;
      flex-direction: column;
      border-radius: var(--r-lg);
      background: var(--bg-2);
      overflow: hidden;
    }
    .chart {
      display: flex;
      align-items: center;
      gap: 10px;
      height: 64px;
      padding: 0 12px 0 8px;
      text-align: left;
      color: var(--fg-3);
      position: relative;
      transition: background-color var(--t-fast) var(--ease);
      animation: rise var(--t-slow) var(--ease-out) backwards;
      animation-delay: var(--d, 0ms);
    }
    .chart:active {
      background: var(--press);
    }
    .chart + .chart::before {
      content: '';
      position: absolute;
      left: 94px;
      right: 0;
      top: 0;
      height: 0.5px;
      background: var(--line);
    }
    .chart-art {
      position: relative;
      width: 48px;
      height: 48px;
      border-radius: var(--r-sm);
      overflow: hidden;
      background: linear-gradient(150deg, var(--bg-3), var(--bg-2));
      flex: none;
      box-shadow: inset 0 0 0 0.5px var(--line);
    }
    .chart-art img {
      width: 100%;
      height: 100%;
      object-fit: cover;
    }
    .chart-rank {
      width: 18px;
      flex: none;
      font-size: var(--fs-footnote);
      font-weight: 600;
      color: var(--fg-3);
      text-align: center;
    }
    .chart:nth-child(-n + 3) .chart-rank {
      color: var(--fg);
    }
    .chart-txt {
      flex: 1;
      min-width: 0;
      display: flex;
      flex-direction: column;
      gap: 1px;
    }
    .chart-name {
      font-size: var(--fs-callout);
      font-weight: 500;
      color: var(--fg);
      letter-spacing: -0.01em;
    }
    .chart-sub {
      font-size: var(--fs-caption);
      color: var(--fg-2);
    }
  }
</style>
