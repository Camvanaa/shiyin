<script lang="ts">
  import Icon from './Icon.svelte';
  import { app } from '../lib/app.svelte';

  let currentTab = $derived.by(() => {
    if (app.sheet === 'downloads') return 'downloads';
    if (app.sheet === 'settings' || app.sheet === 'login') return 'settings';
    if (app.view.kind === 'search') return 'search';
    return 'home';
  });

  function selectTab(tab: 'home' | 'search' | 'downloads' | 'settings') {
    if (tab === 'home') {
      app.sheet = null;
      app.home();
    } else if (tab === 'search') {
      app.sheet = null;
      if (app.view.kind !== 'search') {
        app.search('');
      }
    } else if (tab === 'downloads') {
      app.sheet = 'downloads';
    } else if (tab === 'settings') {
      app.sheet = 'settings';
    }
  }
</script>

<nav class="tabbar" aria-label="移动端主导航">
  <button
    class="tab"
    class:active={currentTab === 'home'}
    onclick={() => selectTab('home')}
    aria-label="发现"
  >
    <Icon name="compass" size={22} />
    <span>发现</span>
  </button>

  <button
    class="tab"
    class:active={currentTab === 'search'}
    onclick={() => selectTab('search')}
    aria-label="搜索"
  >
    <Icon name="search" size={22} />
    <span>搜索</span>
  </button>

  <button
    class="tab"
    class:active={currentTab === 'downloads'}
    onclick={() => selectTab('downloads')}
    aria-label="下载"
  >
    <div class="icon-badge-wrap">
      <Icon name="download" size={22} />
      {#if app.activeJobs > 0}
        <span class="badge-dot">{app.activeJobs}</span>
      {/if}
    </div>
    <span>下载</span>
  </button>

  <button
    class="tab"
    class:active={currentTab === 'settings'}
    onclick={() => selectTab('settings')}
    aria-label="设置"
  >
    <Icon name="settings" size={22} />
    <span>设置</span>
  </button>
</nav>

<style>
  .tabbar {
    position: fixed;
    left: 0;
    right: 0;
    bottom: 0;
    z-index: 60;
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    height: calc(50px + var(--sab));
    padding-bottom: var(--sab);
    background: var(--bg-elev);
    border-top: 0.5px solid var(--line);
    user-select: none;
    -webkit-user-select: none;
  }

  .tab {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 3px;
    height: 50px;
    color: var(--fg-3);
    transition: color var(--t-fast) var(--ease);
    -webkit-tap-highlight-color: transparent;
  }
  .tab span {
    font-size: 10px;
    font-weight: 500;
  }

  .tab.active {
    color: var(--accent);
  }
  .tab:active {
    transform: scale(0.94);
  }

  .icon-badge-wrap {
    position: relative;
    display: grid;
    place-items: center;
  }
  .badge-dot {
    position: absolute;
    top: -3px;
    right: -7px;
    min-width: 14px;
    height: 14px;
    padding: 0 3px;
    border-radius: 99px;
    background: var(--accent);
    color: #fff;
    font-size: 9px;
    font-weight: 700;
    line-height: 14px;
    text-align: center;
  }
</style>
