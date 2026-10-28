<script lang="ts">
  import { fade } from '../lib/fade';
  import Icon from './Icon.svelte';
  import SearchBox from './SearchBox.svelte';
  import { app } from '../lib/app.svelte';
  import { history, clearHistory, type HistoryEntry } from '../lib/history.svelte';
  import { img } from '../lib/format';

  const recentCollections = $derived(history.items.filter((h) => h.kind !== 'query').slice(0, 8));
  const recentQueries = $derived(history.items.filter((h) => h.kind === 'query').slice(0, 10));

  const charts = [
    { id: 3778678, name: '热歌榜' },
    { id: 3779629, name: '新歌榜' },
    { id: 19723756, name: '飙升榜' },
    { id: 2884035, name: '原创榜' },
  ];

  const greeting = (() => {
    const h = new Date().getHours();
    return h < 5 ? '夜深了' : h < 11 ? '早上好' : h < 14 ? '中午好' : h < 18 ? '下午好' : '晚上好';
  })();

  const kindLabel = { song: '歌曲', playlist: '歌单', album: '专辑', query: '' } as const;
</script>

<div class="home">
  <div class="hero">
    <h1>{greeting}{#if app.account}，{app.account.nickname}{/if}</h1>
    <p class="muted">搜索、播放、下载网易云音乐。粘贴链接可直接打开歌曲、歌单或专辑。</p>
    <div class="search"><SearchBox large autofocus /></div>
  </div>

  {#if recentQueries.length}
    <section>
      <div class="sec-head">
        <h2>最近搜索</h2>
        <button class="btn plain" onclick={clearHistory}>清除</button>
      </div>
      <div class="chips">
        {#each recentQueries as q, i (q.at)}
          <button class="chip" style:--d="{i * 20}ms" onclick={() => app.submit(q.q)}>{q.title}</button>
        {/each}
      </div>
    </section>
  {/if}

  {#if recentCollections.length}
    <section>
      <div class="sec-head"><h2>最近打开</h2></div>
      <div class="grid">
        {#each recentCollections as h, i (h.at)}
          {@render card(h, i)}
        {/each}
      </div>
    </section>
  {/if}

  <section>
    <div class="sec-head"><h2>榜单</h2></div>
    <div class="charts">
      {#each charts as c, i}
        <button class="chart" style:--d="{i * 30}ms" onclick={() => app.openCollection('playlist', c.id)}>
          <span class="chart-name">{c.name}</span>
          <Icon name="chevronRight" />
        </button>
      {/each}
    </div>
  </section>
</div>

{#snippet card(h: HistoryEntry, i: number)}
  <button class="card" style:--d="{i * 30}ms" onclick={() => app.submit(h.q)}>
    <span class="cover">
      {#if h.cover}<img use:fade src={img(h.cover, 160)} alt="" loading="lazy" decoding="async" />{/if}
    </span>
    <span class="card-title ellipsis">{h.title}</span>
    <span class="card-sub">{kindLabel[h.kind]}</span>
  </button>
{/snippet}

<style>
  .home {
    padding: 8vh 8px 40px;
  }
  .hero {
    position: relative;
    z-index: 10;
    max-width: 640px;
    margin: 0 auto 48px;
    text-align: center;
    animation: rise var(--t-slow) var(--ease-out) backwards;
  }
  h1 {
    margin: 0;
    font: 700 34px/1.15 var(--font-display);
    letter-spacing: -0.03em;
  }
  .hero p {
    margin: 10px 0 28px;
    font-size: 15px;
  }
  .search {
    text-align: left;
  }
  section {
    margin-top: 36px;
    animation: rise var(--t-slow) var(--ease-out) 60ms backwards;
  }
  .sec-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    margin: 0 0 12px;
    padding: 0 2px;
  }
  h2 {
    margin: 0;
    font: 600 20px var(--font-display);
    letter-spacing: -0.02em;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .chip {
    height: 32px;
    padding: 0 14px;
    border-radius: 999px;
    background: var(--bg-2);
    font-size: 13px;
    max-width: 260px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    animation: rise var(--t) var(--ease-out) backwards;
    animation-delay: var(--d);
    transition:
      background-color var(--t-fast) var(--ease),
      transform var(--t-fast) var(--ease);
  }
  .chip:hover {
    background: var(--bg-3);
  }
  .chip:active {
    transform: scale(0.96);
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
    gap: 20px 16px;
  }
  .card {
    display: flex;
    flex-direction: column;
    text-align: left;
    min-width: 0;
    animation: rise var(--t-slow) var(--ease-out) backwards;
    animation-delay: var(--d);
  }
  .cover {
    aspect-ratio: 1;
    border-radius: var(--r);
    overflow: hidden;
    background: var(--bg-3);
    box-shadow: var(--shadow-sm);
    transition: transform var(--t) var(--ease);
  }
  .card:hover .cover {
    transform: translateY(-2px);
  }
  .card:active .cover {
    transform: scale(0.98);
  }
  .cover img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .card-title {
    margin-top: 8px;
    font-size: 13px;
    font-weight: 500;
  }
  .card-sub {
    font-size: 12px;
    color: var(--fg-2);
  }
  .charts {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: 10px;
  }
  .chart {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 56px;
    padding: 0 16px 0 18px;
    border-radius: var(--r-lg);
    background: var(--bg-2);
    color: var(--fg);
    animation: rise var(--t-slow) var(--ease-out) backwards;
    animation-delay: var(--d);
    transition:
      background-color var(--t-fast) var(--ease),
      transform var(--t-fast) var(--ease);
  }
  .chart:hover {
    background: var(--bg-3);
  }
  .chart:active {
    transform: scale(0.98);
  }
  .chart-name {
    font-size: 15px;
    font-weight: 600;
    letter-spacing: -0.01em;
  }
  .chart :global(svg) {
    width: 16px;
    height: 16px;
    color: var(--fg-3);
  }
</style>
