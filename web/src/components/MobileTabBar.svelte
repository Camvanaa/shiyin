<script lang="ts">
  import Icon from './Icon.svelte';
  import { app } from '../lib/app.svelte';

  type Tab = 'home' | 'search' | 'downloads' | 'settings';

  const current = $derived.by<Tab>(() => {
    if (app.sheet === 'downloads') return 'downloads';
    if (app.sheet === 'settings') return 'settings';
    if (app.view.kind === 'search' || app.view.kind === 'search-home') return 'search';
    return 'home';
  });

  function select(tab: Tab) {
    switch (tab) {
      case 'home':
        app.home();
        break;
      case 'search':
        app.openSearch();
        break;
      case 'downloads':
        app.sheet = app.sheet === 'downloads' ? null : 'downloads';
        break;
      case 'settings':
        app.sheet = app.sheet === 'settings' ? null : 'settings';
        break;
    }
  }

  const tabs: { id: Tab; label: string; icon: 'compass' | 'search' | 'download' | 'settings' }[] = [
    { id: 'home', label: '发现', icon: 'compass' },
    { id: 'search', label: '搜索', icon: 'search' },
    { id: 'downloads', label: '下载', icon: 'download' },
    { id: 'settings', label: '设置', icon: 'settings' },
  ];
</script>

<nav class="tabbar" aria-label="主导航">
  {#each tabs as t (t.id)}
    <button class="tab" class:active={current === t.id} onclick={() => select(t.id)} aria-label={t.label} aria-current={current === t.id ? 'page' : undefined}>
      <span class="ico">
        <Icon name={t.icon} size={23} stroke={current === t.id ? 2.1 : 1.75} />
        {#if t.id === 'downloads' && app.activeJobs > 0}
          <span class="badge-dot tabular">{app.activeJobs}</span>
        {/if}
      </span>
      <span class="label">{t.label}</span>
    </button>
  {/each}
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
    height: calc(var(--tabbar) + var(--sab));
    padding-bottom: var(--sab);
    background: color-mix(in srgb, var(--bg-elev) 92%, transparent);
    -webkit-backdrop-filter: saturate(180%) blur(20px);
    backdrop-filter: saturate(180%) blur(20px);
    border-top: 0.5px solid var(--line);
    user-select: none;
    -webkit-user-select: none;
  }
  .tab {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 2px;
    height: var(--tabbar);
    color: var(--fg-3);
    transition: color var(--t-fast) var(--ease);
  }
  .ico {
    position: relative;
    display: grid;
    place-items: center;
    height: 26px;
    transition: transform var(--t) var(--ease-spring);
  }
  .tab:active .ico {
    transform: scale(0.88);
  }
  .label {
    font-size: 10px;
    font-weight: 500;
    letter-spacing: 0.01em;
  }
  .tab.active {
    color: var(--accent);
  }
  .badge-dot {
    position: absolute;
    top: -2px;
    right: -9px;
    min-width: 15px;
    height: 15px;
    padding: 0 4px;
    border-radius: 99px;
    background: var(--accent);
    color: #fff;
    font-size: 9px;
    font-weight: 700;
    line-height: 15px;
    text-align: center;
    box-shadow: 0 0 0 1.5px var(--bg-elev);
  }
</style>
