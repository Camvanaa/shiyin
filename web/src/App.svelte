<script lang="ts">
  import Icon from './components/Icon.svelte';
  import SearchBox from './components/SearchBox.svelte';
  import Home from './components/Home.svelte';
  import CollectionView from './components/CollectionView.svelte';
  import SongView from './components/SongView.svelte';
  import PlayerBar from './components/PlayerBar.svelte';
  import NowPlaying from './components/NowPlaying.svelte';
  import Downloads from './components/Downloads.svelte';
  import Queue from './components/Queue.svelte';
  import Settings from './components/Settings.svelte';
  import Login from './components/Login.svelte';
  import TokenPrompt from './components/TokenPrompt.svelte';
  import Toasts from './components/Toasts.svelte';
  import { app } from './lib/app.svelte';
  import { player } from './lib/player.svelte';
  import { year } from './lib/format';

  let scroller = $state<HTMLElement>();

  app.init();

  // 视图切换时回到顶部
  $effect(() => {
    void app.viewKey;
    scroller?.scrollTo({ top: 0 });
  });

  let scrolled = $state(false);
  function onScroll() {
    const s = (scroller?.scrollTop ?? 0) > 4;
    if (s !== scrolled) scrolled = s;
  }

  function onKey(e: KeyboardEvent) {
    const typing = (e.target as HTMLElement)?.closest('input, textarea, select, [contenteditable]');
    if (typing) return;
    if (e.code === 'Space' && player.current) {
      e.preventDefault();
      player.toggle();
    } else if ((e.metaKey || e.ctrlKey) && e.key === ',') {
      e.preventDefault();
      app.sheet = 'settings';
    } else if (e.altKey && e.key === 'ArrowLeft') {
      app.back();
    } else if ((e.metaKey || e.ctrlKey) && e.key === 'ArrowRight') {
      player.next();
    } else if ((e.metaKey || e.ctrlKey) && e.key === 'ArrowLeft') {
      player.prev();
    }
  }

  const v = $derived(app.view);
</script>

<svelte:window onkeydown={onKey} />

<div class="shell" class:has-player={!!player.current}>
  <header class="top" class:scrolled>
    <div class="top-left">
      {#if v.kind !== 'home'}
        <button class="icon-btn" onclick={() => app.back()} aria-label="返回" title="返回"><Icon name="back" stroke={2} /></button>
      {/if}
      <button class="brand" onclick={() => app.home()} aria-label="首页">
        <img src="/icon.svg" alt="" width="22" height="22" />
        <span>拾音</span>
      </button>
    </div>
    <div class="top-mid">
      <SearchBox />
    </div>
    <div class="top-right">
      <button class="icon-btn dl" class:on={app.sheet === 'downloads'} onclick={() => (app.sheet = 'downloads')} aria-label="下载" title="下载">
        <Icon name="download" />
        {#if app.activeJobs}<span class="count tabular">{app.activeJobs}</span>{/if}
      </button>
      <button class="icon-btn" onclick={() => (app.sheet = 'settings')} aria-label="设置" title="设置 (⌘,)"><Icon name="settings" /></button>
      <button class="avatar" onclick={() => (app.sheet = 'login')} aria-label="账号" title={app.account?.nickname ?? '登录'}>
        {#if app.account}
          <img src={app.account.avatar + '?param=64y64'} alt="" width="28" height="28" />
        {:else}
          <Icon name="user" />
        {/if}
      </button>
    </div>
  </header>

  <main bind:this={scroller} onscroll={onScroll}>
    <div class="page">
      {#if app.error}
        <div class="error">
          <span>{app.error}</span>
          <button class="btn plain" onclick={() => (app.error = '')}>关闭</button>
        </div>
      {/if}

      {#key app.viewKey}
        <div class="view">
          {#if v.kind === 'home'}
            <Home />
          {:else if v.kind === 'search'}
            <CollectionView
              kind="search"
              eyebrow="搜索结果"
              title={`“${v.q}”`}
              tracks={v.tracks}
              {scroller}
              onreachend={() => app.searchMore()}
            >
              {#snippet footer()}
                <div class="more">
                  {#if app.loading}<span class="spinner"></span>{:else if v.done}<span class="faint">没有更多了</span>{/if}
                </div>
              {/snippet}
            </CollectionView>
          {:else if v.kind === 'song'}
            <SongView track={v.track} />
          {:else if v.kind === 'collection'}
            {@const c = v.data}
            <CollectionView
              kind="collection"
              eyebrow={c.kind === 'album' ? '专辑' : '歌单'}
              title={c.name}
              owner={c.owner}
              meta={c.kind === 'album' ? year(c.publishTime) : ''}
              cover={c.cover}
              description={c.description}
              tracks={c.tracks}
              isAlbum={c.kind === 'album'}
              {scroller}
            />
          {/if}
        </div>
      {/key}
    </div>
  </main>

  <PlayerBar />
</div>

<NowPlaying />
<Downloads />
<Queue />
<Settings />
<Login />
<TokenPrompt />
<Toasts />

<style>
  .shell {
    height: 100%;
    display: grid;
    grid-template-rows: var(--topbar) minmax(0, 1fr) auto;
  }
  .top {
    position: relative;
    z-index: 20;
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 480px) minmax(0, 1fr);
    align-items: center;
    gap: 16px;
    padding: 0 12px 0 14px;
    background: var(--bg);
    border-bottom: 0.5px solid transparent;
    transition: border-color var(--t) var(--ease);
  }
  .top.scrolled {
    border-bottom-color: var(--line);
  }
  .top-left {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 32px;
    padding: 0 8px 0 4px;
    border-radius: var(--r-sm);
    font: 600 15px var(--font-display);
    letter-spacing: -0.01em;
  }
  .brand img {
    border-radius: 6px;
  }
  .top-mid {
    min-width: 0;
  }
  .top-right {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 4px;
  }
  .dl {
    position: relative;
  }
  .count {
    position: absolute;
    top: 2px;
    right: 0;
    min-width: 15px;
    height: 15px;
    padding: 0 4px;
    border-radius: 99px;
    background: var(--accent);
    color: #fff;
    font-size: 10px;
    font-weight: 600;
    line-height: 15px;
    animation: pop var(--t) var(--ease) both;
  }
  @keyframes pop {
    from {
      transform: scale(0.4);
      opacity: 0;
    }
  }
  .avatar {
    width: 32px;
    height: 32px;
    margin-left: 4px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    overflow: hidden;
    background: var(--bg-2);
    color: var(--fg-2);
    transition: transform var(--t-fast) var(--ease);
  }
  .avatar:active {
    transform: scale(0.92);
  }
  .avatar img {
    width: 28px;
    height: 28px;
    border-radius: 50%;
  }
  .avatar :global(svg) {
    width: 18px;
    height: 18px;
  }
  main {
    overflow-y: auto;
    overflow-x: hidden;
    scrollbar-gutter: stable;
  }
  .page {
    max-width: var(--content);
    margin: 0 auto;
    padding: 12px 24px 48px;
  }
  .view {
    animation: rise var(--t) var(--ease-out) backwards;
  }
  .error {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin: 8px 0 16px;
    padding: 10px 10px 10px 16px;
    border-radius: var(--r);
    background: var(--accent-soft);
    color: var(--accent);
    font-size: 13px;
    animation: rise var(--t) var(--ease-out);
  }
  .more {
    display: grid;
    place-items: center;
    height: 64px;
    font-size: 13px;
  }

  @media (max-width: 640px) {
    .top {
      grid-template-columns: auto minmax(0, 1fr) auto;
      gap: 8px;
    }
    .brand span {
      display: none;
    }
    .page {
      padding: 8px 12px 32px;
    }
  }
</style>
