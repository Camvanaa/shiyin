<script lang="ts">
  import Icon from './Icon.svelte';
  import type { IconName } from './Icon.svelte';
  import { app } from '../lib/app.svelte';
  import { sourceShort } from '../lib/format';

  // 桌面侧边栏（Apple Music for Mac 式）。窄屏（< 1000px）折叠为图标栏，
  // < 900px 整体隐藏，由顶栏图标接管导航。
  type Item = { id: string; label: string; icon: IconName; on: () => void; badge?: number };

  const sectionA = $derived<Item[]>([
    { id: 'home', label: '主页', icon: 'compass', on: () => app.home() },
    { id: 'search', label: '搜索', icon: 'search', on: () => app.openSearch() },
  ]);
  const sectionB = $derived<Item[]>([
    { id: 'library', label: '资料库', icon: 'library', on: () => app.openLibrary() },
    { id: 'history', label: '最近播放', icon: 'clock', on: () => app.openHistory() },
    { id: 'downloads', label: '下载', icon: 'download', on: () => app.openTab('downloads'), badge: app.activeJobs },
  ]);

  const cur = $derived(app.view.kind);
  const isOn = (id: string) =>
    id === 'search' ? cur === 'search' || cur === 'search-home' : id === 'library' ? cur === 'library' || cur === 'playlist' : cur === id;
</script>

<aside class="sidebar" aria-label="侧边导航">
  <button class="brand" onclick={() => app.home()} aria-label="主页">
    <img src="/icon.svg" alt="" width="24" height="24" />
    <span class="name">拾音</span>
  </button>

  <nav class="nav">
    {#each sectionA as it (it.id)}
      <button class="item" class:on={isOn(it.id)} onclick={it.on} aria-current={isOn(it.id) ? 'page' : undefined}>
        <span class="ico"><Icon name={it.icon} size={19} stroke={isOn(it.id) ? 2.1 : 1.75} /></span>
        <span class="lbl">{it.label}</span>
      </button>
    {/each}
  </nav>

  <div class="cap">我的</div>
  <nav class="nav">
    {#each sectionB as it (it.id)}
      <button class="item" class:on={isOn(it.id)} onclick={it.on} aria-current={isOn(it.id) ? 'page' : undefined}>
        <span class="ico">
          <Icon name={it.icon} size={19} stroke={isOn(it.id) ? 2.1 : 1.75} />
          {#if it.badge}<span class="badge tabular">{it.badge}</span>{/if}
        </span>
        <span class="lbl">{it.label}</span>
      </button>
    {/each}
  </nav>

  <div class="spacer"></div>

  {#if app.account || app.sources.length}
    <div class="foot">
      <button class="acc" onclick={() => (app.sheet = 'login')} aria-label="账号设置">
        {#if app.account}<span class="acc-name ellipsis">{app.account.nickname}</span>{/if}
        <span class="acc-src ellipsis">{app.sources.map((s) => sourceShort(s.id)).join(' · ') || '未登录'}</span>
      </button>
      <button class="icon-btn" onclick={() => (app.sheet = 'settings')} aria-label="设置" title="设置 (⌘,)"><Icon name="settings" size={18} /></button>
    </div>
  {/if}
</aside>

<style>
  .sidebar {
    grid-area: side;
    flex: 1;
    width: 100%;
    display: flex;
    flex-direction: column;
    min-height: 0;
    padding: 10px 10px 8px;
    gap: 2px;
    background: var(--bg-2);
    border-right: 0.5px solid var(--line);
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 9px;
    height: 40px;
    padding: 0 8px;
    border-radius: var(--r-md);
    font: 600 var(--fs-headline) var(--font-display);
    letter-spacing: -0.015em;
    transition: background-color var(--t-fast) var(--ease);
  }
  .brand:hover {
    background: var(--hover);
  }
  .brand img {
    border-radius: var(--r-sm);
    flex: none;
  }
  .nav {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .item {
    display: flex;
    align-items: center;
    gap: 11px;
    height: 36px;
    padding: 0 8px;
    border-radius: var(--r-md);
    color: var(--fg-2);
    font-size: var(--fs-subhead);
    font-weight: 500;
    text-align: left;
    transition:
      background-color var(--t-fast) var(--ease),
      color var(--t-fast) var(--ease);
  }
  .item:hover {
    background: var(--hover);
    color: var(--fg);
  }
  .item.on {
    background: var(--bg-elev);
    color: var(--fg);
    box-shadow: var(--shadow-sm);
  }
  .ico {
    position: relative;
    display: grid;
    place-items: center;
    width: 20px;
    flex: none;
  }
  .item.on .ico {
    color: var(--accent);
  }
  .lbl {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .cap {
    margin: 16px 8px 4px;
    font-size: var(--fs-caption2);
    font-weight: 600;
    letter-spacing: 0.02em;
    color: var(--fg-3);
  }
  .badge {
    position: absolute;
    top: -4px;
    right: -6px;
    min-width: 15px;
    height: 15px;
    padding: 0 4px;
    border-radius: var(--r-pill);
    background: var(--accent);
    color: var(--white);
    font-size: var(--fs-micro);
    font-weight: 600;
    line-height: 15px;
    text-align: center;
    box-shadow: 0 0 0 1.5px var(--bg-2);
  }
  .spacer {
    flex: 1;
    min-height: 8px;
  }
  .foot {
    display: flex;
    align-items: center;
    gap: 6px;
    padding-top: 6px;
    border-top: 0.5px solid var(--line);
  }
  .acc {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 1px;
    padding: 5px 8px;
    border-radius: var(--r-md);
    text-align: left;
    transition: background-color var(--t-fast) var(--ease);
  }
  .acc:hover {
    background: var(--hover);
  }
  .acc-name {
    max-width: 100%;
    font-size: var(--fs-footnote);
    font-weight: 600;
    color: var(--fg);
  }
  .acc-src {
    max-width: 100%;
    font-size: var(--fs-caption2);
    color: var(--fg-3);
  }

  /* 窄桌面（900–1080px）：折叠为图标栏 */
  @media (min-width: 900px) and (max-width: 1080px) {
    .sidebar {
      padding-left: 8px;
      padding-right: 8px;
      align-items: center;
    }
    .brand {
      padding: 0;
      justify-content: center;
    }
    .name,
    .lbl,
    .acc-name,
    .acc-src {
      display: none;
    }
    .cap {
      margin: 14px 0 4px;
      text-align: center;
      font-size: var(--fs-micro);
    }
    .item {
      justify-content: center;
      padding: 0;
      width: 40px;
    }
    .acc {
      align-items: center;
      padding: 6px 0;
    }
  }
</style>
