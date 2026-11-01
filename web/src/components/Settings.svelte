<script lang="ts">
  import Sheet from './Sheet.svelte';
  import Icon from './Icon.svelte';
  import SettingsBody, { type Pane } from './SettingsBody.svelte';
  import { app } from '../lib/app.svelte';
  import { isTauri } from '../lib/transport';

  // 桌面：恒定尺寸面板 + 左侧分类导航。移动端的设置是独立页面（SettingsPage）。
  const panes: { id: Pane; label: string; icon: 'note' | 'download' | 'list' | 'disc' | 'mini' | 'settings' }[] = [
    { id: 'quality', label: '音质与播放', icon: 'note' },
    { id: 'download', label: '下载', icon: 'download' },
    { id: 'naming', label: '文件名', icon: 'list' },
    { id: 'meta', label: '元数据', icon: 'disc' },
    ...(isTauri ? [{ id: 'desktop' as Pane, label: '桌面', icon: 'mini' as const }] : []),
    { id: 'general', label: '通用', icon: 'settings' },
  ];
  let pane = $state<Pane>('quality');
</script>

<Sheet open={app.sheet === 'settings'} onclose={() => (app.sheet = null)} title="设置" width={720} size="lg" bare>
  <div class="layout">
    <nav aria-label="设置分类">
      {#each panes as p (p.id)}
        <button class="nav-item" class:on={pane === p.id} onclick={() => (pane = p.id)} aria-current={pane === p.id ? 'true' : undefined}>
          <Icon name={p.icon} size={17} />
          <span>{p.label}</span>
        </button>
      {/each}
      <div class="nav-foot">
        <p class="ver">拾音 0.1.0</p>
        <p class="ver faint">GPL-3.0</p>
      </div>
    </nav>
    <div class="pane">
      {#key pane}
        <div class="pane-in"><SettingsBody {pane} /></div>
      {/key}
    </div>
  </div>
</Sheet>

<style>
  .layout {
    display: grid;
    grid-template-columns: 200px minmax(0, 1fr);
    height: 100%;
    min-height: 0;
  }
  nav {
    display: flex;
    flex-direction: column;
    gap: 1px;
    padding: 6px 8px 10px;
    background: color-mix(in srgb, var(--bg-2) 60%, transparent);
    border-right: 0.5px solid var(--line);
    overflow: auto;
  }
  .nav-item {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 34px;
    padding: 0 10px;
    border-radius: 7px;
    font-size: 13px;
    font-weight: 500;
    color: var(--fg-2);
    text-align: left;
    transition: background-color var(--t-fast) var(--ease), color var(--t-fast) var(--ease);
  }
  .nav-item :global(svg) {
    width: 17px;
    height: 17px;
    flex: none;
    opacity: 0.85;
  }
  .nav-item:hover {
    background: var(--hover);
    color: var(--fg);
  }
  .nav-item.on {
    background: var(--accent-soft);
    color: var(--accent);
  }
  .nav-item.on :global(svg) {
    opacity: 1;
  }
  .nav-foot {
    margin-top: auto;
    padding: 10px;
  }
  .ver {
    margin: 0;
    font-size: 11px;
    color: var(--fg-3);
  }
  .pane {
    min-height: 0;
    overflow: auto;
    padding: 18px 20px 24px;
    overscroll-behavior: contain;
  }
  .pane-in {
    animation: fade var(--t) var(--ease) both;
  }
</style>
