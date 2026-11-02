<script lang="ts">
  import Icon from './components/Icon.svelte';
  import SearchBox from './components/SearchBox.svelte';
  import SourceFilter from './components/SourceFilter.svelte';
  import Sidebar from './components/Sidebar.svelte';
  import Home from './components/Home.svelte';
  import CollectionView from './components/CollectionView.svelte';
  import SongView from './components/SongView.svelte';
  import PlayerBar from './components/PlayerBar.svelte';
  import TokenPrompt from './components/TokenPrompt.svelte';
  import Toasts from './components/Toasts.svelte';
  import MobileTabBar from './components/MobileTabBar.svelte';
  import MobilePlayerPill from './components/MobilePlayerPill.svelte';
  import MobileSearch from './components/MobileSearch.svelte';
  import RecentView from './components/RecentView.svelte';
  import SettingsPage from './components/SettingsPage.svelte';
  import DownloadsPage from './components/DownloadsPage.svelte';

  import { app } from './lib/app.svelte';
  import { player } from './lib/player.svelte';
  import { collectionKindLabel, sourceName, year } from './lib/format';

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
  /** 有全屏浮层时，主界面整体 inert，键盘/读屏不会跑到浮层后面 */
  const modalOpen = $derived(!!app.sheet || player.expanded);

  // 只在首次需要时才加载浮层组件（首屏包瘦身）。加载后保持挂载，退场动画才完整。
  const lazy = $state({ login: false, settings: false, downloads: false, queue: false, nowPlaying: false });
  $effect(() => {
    const s = app.sheet;
    if (s && s !== 'token') lazy[s] = true;
    if (player.expanded) lazy.nowPlaying = true;
  });
  // 空闲时预热：展开播放页/打开设置不会看到加载延迟
  $effect(() => {
    const idle = () => {
      import('./components/NowPlaying.svelte');
      import('./components/Settings.svelte');
      import('./components/Login.svelte');
      import('./components/Downloads.svelte');
      import('./components/Queue.svelte');
    };
    const w = globalThis as unknown as { requestIdleCallback?: (cb: () => void, o?: { timeout: number }) => number };
    if (w.requestIdleCallback) w.requestIdleCallback(idle, { timeout: 3000 });
    else setTimeout(idle, 2500);
  });
  const showBack = $derived(!['home', 'search-home', 'settings', 'downloads'].includes(v.kind));
  /** 自带大标题的页面：顶栏小标题仅在滚动后出现 */
  const hasLargeTitle = $derived(['home', 'settings', 'downloads'].includes(v.kind));
  const mobileTitle = $derived.by(() => {
    switch (v.kind) {
      case 'home':
        return '主页';
      case 'search-home':
      case 'search':
        return '搜索';
      case 'song':
        return '歌曲';
      case 'collection':
        return collectionKindLabel(v.data.kind);
      case 'history':
        return '最近播放';
      case 'settings':
        return '设置';
      case 'downloads':
        return '下载';
    }
  });
</script>

<svelte:window onkeydown={onKey} />

<div class="shell" class:has-player={!!player.current} inert={modalOpen || undefined}>
  <div class="side-slot">
    <Sidebar />
  </div>
  <div class="col">
  <header class="top" class:scrolled>
    {#if app.loading}<div class="loadbar" role="progressbar" aria-label="加载中"></div>{/if}
    <div class="top-left">
      {#if showBack}
        <button class="icon-btn" onclick={() => app.back()} aria-label="返回" title="返回"><Icon name="back" stroke={2} /></button>
      {/if}
      <button class="brand" onclick={() => app.home()} aria-label="首页">
        <img src="/icon.svg" alt="" width="22" height="22" />
        <span class="brand-name">拾音</span>
      </button>
    </div>

    <div class="top-mid">
      <div class="desktop-only"><SearchBox /></div>
      <div class="mobile-only"><span class="top-title" class:collapsed={hasLargeTitle && !scrolled}>{mobileTitle}</span></div>
    </div>

    <div class="top-right">
      <div class="desktop-only">
        <button class="icon-btn nav-dup" class:on={v.kind === 'history'} onclick={() => app.openHistory()} aria-label="最近播放" title="最近播放">
          <Icon name="clock" />
        </button>
        <button
          class="icon-btn"
          class:on={app.sheet === 'queue'}
          disabled={!player.queue.length}
          onclick={() => (app.sheet = app.sheet === 'queue' ? null : 'queue')}
          aria-label="播放列表"
          title={player.queue.length ? `播放列表 · ${player.queue.length} 首` : '播放列表为空'}
        >
          <Icon name="queue" />
        </button>
        <button class="icon-btn dl nav-dup" class:on={app.sheet === 'downloads'} onclick={() => (app.sheet = 'downloads')} aria-label="下载" title="下载">
          <Icon name="download" />
          {#if app.activeJobs}<span class="count tabular">{app.activeJobs}</span>{/if}
        </button>
        <button class="icon-btn nav-dup" onclick={() => (app.sheet = 'settings')} aria-label="设置" title="设置 (⌘,)"><Icon name="settings" /></button>
      </div>
      <button class="avatar" class:hide-mobile={v.kind === 'settings'} onclick={() => (app.sheet = 'login')} aria-label="账号" title={app.account?.nickname ?? '登录'}>
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
        <div class="error" role="alert">
          <span>{app.error}</span>
          <button class="btn plain" onclick={() => (app.error = '')}>关闭</button>
        </div>
      {/if}

      {#key app.viewKey}
        <div class="view">
          {#if v.kind === 'home'}
            <Home />
          {:else if v.kind === 'search-home'}
            <MobileSearch />
          {:else if v.kind === 'search'}
            <CollectionView
              kind="search"
              eyebrow="搜索结果"
              title={v.q}
              tracks={v.tracks}
              {scroller}
              onreachend={() => app.searchMore()}
            >
              {#snippet subhead()}
                <div class="src-tabs">
                  <SourceFilter />
                </div>
              {/snippet}
              {#snippet footer()}
                <div class="more">
                  {#if app.loading}<span class="spinner"></span>{:else if v.done}<span class="faint">没有更多了</span>{/if}
                </div>
              {/snippet}
            </CollectionView>
          {:else if v.kind === 'song'}
            <SongView track={v.track} />
          {:else if v.kind === 'history'}
            <RecentView {scroller} />
          {:else if v.kind === 'settings'}
            <SettingsPage />
          {:else if v.kind === 'downloads'}
            <DownloadsPage />
          {:else if v.kind === 'collection'}
            {@const c = v.data}
            <CollectionView
              kind="collection"
              eyebrow={`${sourceName(c.source)} · ${collectionKindLabel(c.kind)}`}
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

  <div class="desktop-only"><PlayerBar /></div>
  <div class="mobile-only">
    <MobilePlayerPill />
    <MobileTabBar />
  </div>
  </div>
</div>

{#if lazy.nowPlaying}{#await import('./components/NowPlaying.svelte') then { default: C }}<C />{/await}{/if}
{#if lazy.downloads}{#await import('./components/Downloads.svelte') then { default: C }}<C />{/await}{/if}
{#if lazy.queue}{#await import('./components/Queue.svelte') then { default: C }}<C />{/await}{/if}
{#if lazy.settings}{#await import('./components/Settings.svelte') then { default: C }}<C />{/await}{/if}
{#if lazy.login}{#await import('./components/Login.svelte') then { default: C }}<C />{/await}{/if}
<TokenPrompt />
<Toasts />

<style>
  .shell {
    height: 100%;
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
  }
  .side-slot {
    display: none;
  }
  .col {
    display: grid;
    grid-template-rows: var(--topbar) minmax(0, 1fr) auto;
    min-width: 0;
    min-height: 0;
  }
  .top {
    position: relative;
    z-index: var(--z-sticky);
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
  /* 移动端此前打开歌单/歌曲没有任何加载指示，这里用一个顶部细进度条兜底 */
  .loadbar {
    position: absolute;
    left: 0;
    right: 0;
    bottom: -1px;
    height: 2px;
    overflow: hidden;
    background: var(--accent-soft);
  }
  .loadbar::after {
    content: '';
    position: absolute;
    inset: 0;
    width: 40%;
    border-radius: var(--r-pill);
    background: var(--accent);
    animation: loadslide 1.1s var(--ease-in-out) infinite;
  }
  @keyframes loadslide {
    from {
      transform: translateX(-100%);
    }
    to {
      transform: translateX(250%);
    }
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
    font: 600 var(--fs-callout) var(--font-display);
    letter-spacing: -0.01em;
  }
  .brand img {
    border-radius: var(--r-sm);
  }
  .top-mid {
    min-width: 0;
    display: flex;
    justify-content: center;
  }
  .top-mid > :global(*) {
    width: 100%;
  }
  .top-title {
    display: block;
    text-align: center;
    font: 600 var(--fs-body) var(--font-display);
    letter-spacing: -0.01em;
    color: var(--fg);
    transition: opacity var(--t) var(--ease), transform var(--t) var(--ease);
  }
  .top-title.collapsed {
    opacity: 0;
    transform: translateY(4px);
  }
  .top-right {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 4px;
  }
  .top-right .desktop-only {
    display: flex;
    gap: 4px;
  }
  .top-right .icon-btn:disabled {
    opacity: 0.35;
    pointer-events: none;
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
    border-radius: var(--r-pill);
    background: var(--accent);
    color: var(--white);
    font-size: var(--fs-micro);
    font-weight: 600;
    line-height: 15px;
    animation: pop var(--t) var(--ease-spring) both;
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
    overscroll-behavior-y: contain;
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
    font-size: var(--fs-footnote);
    animation: rise var(--t) var(--ease-out);
  }
  .src-tabs {
    margin-top: 12px;
    max-width: 100%;
  }
  .more {
    display: grid;
    place-items: center;
    height: 64px;
    font-size: var(--fs-footnote);
  }

  /* 桌面（≥900px）：显示侧边栏，顶栏的导航图标让位给侧边栏 */
  @media (min-width: 900px) {
    .side-slot {
      display: flex;
      width: 232px;
    }
    .top-left .brand {
      display: none;
    }
    /* 最近播放 / 下载 / 设置 已在侧边栏，顶栏只留播放列表 */
    .top-right .desktop-only .icon-btn.nav-dup {
      display: none;
    }
    .page {
      padding: 12px 32px 48px;
    }
  }
  @media (min-width: 900px) and (max-width: 1080px) {
    .side-slot {
      width: 64px;
    }
  }

  @media (max-width: 640px) {
    .shell {
      grid-template-columns: minmax(0, 1fr);
    }
    .col {
      grid-template-rows: 48px minmax(0, 1fr);
    }
    .top {
      grid-template-columns: 44px minmax(0, 1fr) 44px;
      gap: 8px;
      padding: 0 8px;
    }
    .top-left {
      justify-content: flex-start;
    }
    .brand-name {
      display: none;
    }
    .top-right .desktop-only {
      display: none;
    }
    .avatar {
      margin-left: 0;
    }
    .avatar.hide-mobile {
      visibility: hidden;
    }
    main {
      scrollbar-gutter: auto;
    }
    .page {
      padding: 4px 12px calc(var(--tabbar) + var(--sab) + 24px);
    }
    .has-player .page {
      padding-bottom: calc(var(--tabbar) + var(--sab) + 56px + 24px);
    }
  }
</style>
