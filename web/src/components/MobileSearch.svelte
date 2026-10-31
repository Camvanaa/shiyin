<script lang="ts">
  import Icon from './Icon.svelte';
  import TrackList from './TrackList.svelte';
  import { app } from '../lib/app.svelte';
  import { player } from '../lib/player.svelte';
  import { history, clearHistory, type HistoryEntry } from '../lib/history.svelte';
  import { img } from '../lib/format';

  let inputVal = $state('');
  let inputEl = $state<HTMLInputElement>();
  let scroller = $state<HTMLElement>();

  const recentQueries = $derived(history.items.filter((h) => h.kind === 'query').slice(0, 10));
  const recentCollections = $derived(history.items.filter((h) => h.kind !== 'query').slice(0, 6));

  const isSearching = $derived(app.view.kind === 'search');
  const searchResults = $derived(app.view.kind === 'search' ? app.view.tracks : []);

  function handleSearch(q = inputVal) {
    const text = q.trim();
    if (!text) return;
    inputVal = text;
    inputEl?.blur();
    app.submit(text);
  }

  function handleClear() {
    inputVal = '';
    inputEl?.focus();
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      e.preventDefault();
      handleSearch();
    }
  }

  const kindLabel = { song: '歌曲', playlist: '歌单', album: '专辑', query: '' } as const;
</script>

<div class="mobile-search" bind:this={scroller}>
  <div class="search-header">
    <div class="input-wrap">
      <Icon name="search" size={17} />
      <input
        bind:this={inputEl}
        bind:value={inputVal}
        onkeydown={onKey}
        placeholder="搜索歌曲，或粘贴歌曲、歌单、专辑链接"
        autocomplete="off"
        spellcheck="false"
        enterkeyhint="search"
      />
      {#if inputVal}
        <button class="clear-btn" onclick={handleClear} aria-label="清空">
          <Icon name="close" size={12} stroke={2.5} />
        </button>
      {/if}
    </div>

    <!-- 音源分段选择器 -->
    <div class="src-selector" role="tablist" aria-label="音源">
      {#each [['all', '全部音源'], ['netease', '网易云'], ['qq', 'QQ音乐'], ['kuwo', '酷我']] as [id, label]}
        <button
          class:active={app.source === id}
          onclick={() => {
            app.setSource(id);
            if (inputVal.trim()) handleSearch();
          }}
          role="tab"
          aria-selected={app.source === id}
        >
          {label}
        </button>
      {/each}
    </div>
  </div>

  <div class="search-body">
    {#if isSearching}
      <div class="results-head">
        <h2>搜索结果</h2>
        <span class="count muted">{searchResults.length} 首歌曲</span>
      </div>
      <TrackList
        tracks={searchResults}
        {scroller}
        showAlbum={false}
        showCover={true}
        onplay={(i) => player.playList(searchResults, i)}
        onreachend={() => app.searchMore()}
      />
      {#if app.loading}
        <div class="loading-state"><span class="spinner"></span></div>
      {/if}
    {:else}
      <!-- 历史搜索 -->
      {#if recentQueries.length}
        <section class="section">
          <div class="sec-title">
            <h3>最近搜索</h3>
            <button class="btn plain sm" onclick={clearHistory}>清除</button>
          </div>
          <div class="chips">
            {#each recentQueries as q}
              <button class="chip" onclick={() => handleSearch(q.q)}>
                {q.title}
              </button>
            {/each}
          </div>
        </section>
      {/if}

      <!-- 最近打开 -->
      {#if recentCollections.length}
        <section class="section">
          <div class="sec-title">
            <h3>最近打开</h3>
          </div>
          <div class="recent-list">
            {#each recentCollections as h}
              <button class="recent-item" onclick={() => app.submit(h.q)}>
                <span class="r-cover">
                  {#if h.cover}
                    <img src={img(h.cover, 56)} alt="" width="48" height="48" loading="lazy" />
                  {:else}
                    <Icon name={h.kind === 'album' ? 'disc' : 'list'} size={20} />
                  {/if}
                </span>
                <span class="r-text">
                  <span class="r-name ellipsis">{h.title}</span>
                  <span class="r-sub">{kindLabel[h.kind]}</span>
                </span>
                <Icon name="chevronRight" size={16} />
              </button>
            {/each}
          </div>
        </section>
      {/if}

      <!-- 快捷热门分类/榜单 -->
      <section class="section">
        <div class="sec-title">
          <h3>热门榜单探索</h3>
        </div>
        <div class="charts-grid">
          {#each [
            { id: '3778678', name: '热歌榜', source: 'netease', sub: '网易云 · 每天更新' },
            { id: '26', name: 'QQ热歌榜', source: 'qq', sub: 'QQ音乐 · 实时更新' },
            { id: '3779629', name: '新歌榜', source: 'netease', sub: '网易云 · 每天更新' },
            { id: '62', name: 'QQ飙升榜', source: 'qq', sub: 'QQ音乐 · 实时更新' },
          ] as c}
            <button
              class="chart-card"
              onclick={() => app.openCollection(c.source === 'qq' ? 'toplist' : 'playlist', c.id, undefined, c.source)}
            >
              <div class="c-info">
                <span class="c-name">{c.name}</span>
                <span class="c-sub">{c.sub}</span>
              </div>
              <Icon name="chevronRight" size={16} />
            </button>
          {/each}
        </div>
      </section>
    {/if}
  </div>
</div>

<style>
  .mobile-search {
    padding-bottom: calc(var(--playbar) + 70px + var(--sab));
  }
  .search-header {
    position: sticky;
    top: 0;
    z-index: 15;
    background: var(--bg);
    padding: 8px 4px 12px;
    border-bottom: 0.5px solid var(--line);
  }
  .input-wrap {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 42px;
    padding: 0 12px;
    border-radius: 12px;
    background: var(--bg-2);
    color: var(--fg-3);
  }
  .input-wrap input {
    flex: 1;
    min-width: 0;
    height: 100%;
    border: 0;
    outline: none;
    background: transparent;
    font-size: 15px;
    color: var(--fg);
  }
  .input-wrap input::placeholder {
    color: var(--fg-3);
    font-size: 14px;
  }
  .clear-btn {
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: var(--fg-3);
    color: var(--bg);
    display: grid;
    place-items: center;
  }

  .src-selector {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 4px;
    margin-top: 10px;
    padding: 3px;
    background: var(--bg-2);
    border-radius: 10px;
  }
  .src-selector button {
    height: 28px;
    font-size: 12px;
    font-weight: 500;
    color: var(--fg-2);
    border-radius: 7px;
    transition: all var(--t-fast) var(--ease);
  }
  .src-selector button.active {
    background: var(--bg-elev);
    color: var(--fg);
    box-shadow: var(--shadow-sm);
  }

  .search-body {
    padding: 16px 4px;
  }
  .results-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    margin-bottom: 12px;
  }
  .results-head h2 {
    margin: 0;
    font: 700 20px var(--font-display);
  }
  .count {
    font-size: 13px;
  }

  .section {
    margin-bottom: 26px;
  }
  .sec-title {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 10px;
  }
  .sec-title h3 {
    margin: 0;
    font: 600 16px var(--font-display);
    color: var(--fg);
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
    color: var(--fg);
    max-width: 220px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .chip:active {
    transform: scale(0.96);
  }

  .recent-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .recent-item {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 10px;
    border-radius: 12px;
    background: var(--bg-2);
    text-align: left;
  }
  .recent-item:active {
    transform: scale(0.98);
  }
  .r-cover {
    width: 44px;
    height: 44px;
    border-radius: 8px;
    overflow: hidden;
    background: var(--bg-3);
    display: grid;
    place-items: center;
    flex: none;
  }
  .r-cover img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .r-text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .r-name {
    font-size: 14px;
    font-weight: 500;
  }
  .r-sub {
    font-size: 12px;
    color: var(--fg-2);
  }

  .charts-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }
  .chart-card {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 14px;
    border-radius: 12px;
    background: var(--bg-2);
    text-align: left;
  }
  .chart-card:active {
    transform: scale(0.98);
  }
  .c-info {
    display: flex;
    flex-direction: column;
  }
  .c-name {
    font-size: 14px;
    font-weight: 600;
  }
  .c-sub {
    font-size: 11px;
    color: var(--fg-3);
    margin-top: 2px;
  }

  .loading-state {
    display: flex;
    justify-content: center;
    padding: 24px 0;
  }
</style>
