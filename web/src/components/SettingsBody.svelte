<script lang="ts">
  import Icon from './Icon.svelte';
  import Select from './Select.svelte';
  import { app } from '../lib/app.svelte';
  import { settings } from '../lib/settings.svelte';
  import { player } from '../lib/player.svelte';
  import { clearHistory } from '../lib/history.svelte';
  import { clearRecent } from '../lib/recent.svelte';
  import { api, isTauri, token } from '../lib/transport';
  import { toast } from '../lib/toast.svelte';
  import { LEVELS } from '../lib/types';

  export type Pane = 'quality' | 'download' | 'naming' | 'meta' | 'desktop' | 'general';

  const THEMES: { value: 'system' | 'light' | 'dark'; label: string }[] = [
    { value: 'system', label: '跟随系统' },
    { value: 'light', label: '浅色' },
    { value: 'dark', label: '深色' },
  ];
  const LIMITS = [20, 30, 50, 100].map((n) => ({ value: n, label: String(n) }));

  // 设置内容：桌面侧栏面板按 pane 单独渲染；移动端页面传 'all' 一屏到底
  let { pane = 'all' }: { pane?: Pane | 'all' } = $props();
  const show = (p: Pane) => pane === 'all' || pane === p;

  let defaultDir = $state('');
  $effect(() => {
    if (isTauri && (app.sheet === 'settings' || app.view.kind === 'settings')) api.defaultDir?.().then((d) => (defaultDir = d));
  });

  const preview = $derived(
    (settings.template.trim() || '${title} - ${artist}')
      .replaceAll('${title}', '晴天')
      .replaceAll('${artist}', '周杰伦')
      .replaceAll('${album}', '叶惠美')
      .replaceAll('${track}', '03')
      .replace(/[\\/:*?"<>|]/g, '_'),
  );

  const presets = ['${title} - ${artist}', '${artist} - ${title}', '${track}. ${title}', '${title}_${artist}_${album}'];

  function insert(v: string) {
    settings.template = settings.template + v;
  }

  async function pickDir() {
    const d = await api.pickDir?.();
    if (d) settings.dir = d;
  }

  let lastPlayLevel = settings.playLevel;
  $effect(() => {
    if (settings.playLevel !== lastPlayLevel) {
      lastPlayLevel = settings.playLevel;
      player.reloadQuality();
    }
  });

  let lastConc = settings.concurrency;
  $effect(() => {
    const c = settings.concurrency;
    if (c !== lastConc) {
      lastConc = c;
      api.setConcurrency(c).catch(() => {});
    }
  });
</script>

{#if show('quality')}
  <section>
    <h3>音质与播放</h3>
    <div class="group">
      <div class="item">
        <span>播放音质</span>
        <Select bind:value={settings.playLevel} options={LEVELS} label="播放音质" />
      </div>
      <div class="item">
        <span>下载音质</span>
        <Select bind:value={settings.level} options={LEVELS} label="下载音质" />
      </div>
      <label class="item">
        <span>无版权或仅试听时自动换源</span>
        <input type="checkbox" class="switch" bind:checked={settings.autoFallback} />
      </label>
    </div>
    <p class="note">{LEVELS.find((l) => l.value === settings.level)?.hint}。高于账号权限时会自动降级；开启自动换源后会在其它平台寻找同名可播版本。</p>
  </section>
{/if}

{#if show('download')}
  <section>
    <h3>下载</h3>
    <div class="group">
      {#if isTauri}
        <div class="item">
          <span>保存位置</span>
          <button class="path" onclick={pickDir} title={settings.dir || defaultDir}>
            <Icon name="folder" /><span class="ellipsis">{settings.dir || defaultDir || '下载'}</span>
          </button>
        </div>
        {#if settings.dir}
          <div class="item sub">
            <span></span><button class="btn plain" onclick={() => (settings.dir = '')}>恢复默认</button>
          </div>
        {/if}
        <label class="item">
          <span>歌单和专辑存入子文件夹</span>
          <input type="checkbox" class="switch" bind:checked={settings.subfolder} />
        </label>
      {/if}
      <label class="item">
        <span>同时下载</span>
        <div class="stepper">
          <button onclick={() => (settings.concurrency = Math.max(1, settings.concurrency - 1))} aria-label="减少">−</button>
          <span class="tabular">{settings.concurrency}</span>
          <button onclick={() => (settings.concurrency = Math.min(10, settings.concurrency + 1))} aria-label="增加">+</button>
        </div>
      </label>
    </div>
    {#if !isTauri}<p class="note">Web 版下载完成后由浏览器保存文件，无法指定目录。</p>{/if}
  </section>
{/if}

{#if show('naming')}
  <section>
    <h3>文件名</h3>
    <div class="group">
      <div class="item col">
        <input class="input mono" bind:value={settings.template} placeholder={'${title} - ${artist}'} spellcheck="false" />
        <div class="tokens">
          {#each [['${title}', '歌名'], ['${artist}', '歌手'], ['${album}', '专辑'], ['${track}', '曲目号']] as [v, l]}
            <button class="token" onclick={() => insert(v)}>{l}</button>
          {/each}
          <span class="sep"></span>
          {#each presets as p}
            <button class="token ghost" class:on={settings.template === p} onclick={() => (settings.template = p)}>
              {p.replaceAll('${title}', '歌名').replaceAll('${artist}', '歌手').replaceAll('${album}', '专辑').replaceAll('${track}', '序号')}
            </button>
          {/each}
        </div>
        <div class="preview">预览：<span class="mono">{preview}.mp3</span></div>
      </div>
    </div>
  </section>
{/if}

{#if show('meta')}
  <section>
    <h3>元数据</h3>
    <div class="group">
      <label class="item">
        <span>写入元数据</span>
        <input type="checkbox" class="switch" bind:checked={settings.writeMeta} />
      </label>
      <div class="sub-items" class:disabled={!settings.writeMeta}>
        {#each [['writeCover', '封面'], ['compressCover', '封面压缩为 640px'], ['writeArtist', '歌手'], ['writeAlbum', '专辑'], ['writeTrack', '曲目号'], ['writeLyrics', '歌词（含翻译）']] as const as [k, l]}
          <label class="item">
            <span>{l}</span>
            <input type="checkbox" class="switch" bind:checked={settings[k]} disabled={!settings.writeMeta} />
          </label>
        {/each}
      </div>
      <label class="item">
        <span>同时保存 .lrc 歌词文件</span>
        <input type="checkbox" class="switch" bind:checked={settings.lrcFile} />
      </label>
    </div>
  </section>
{/if}

{#if isTauri && show('desktop')}
  <section>
    <h3>桌面</h3>
    <div class="group">
      <label class="item">
        <span>关闭窗口时保留在托盘</span>
        <input type="checkbox" class="switch" bind:checked={settings.closeToTray} />
      </label>
      <label class="item">
        <span>全局快捷键</span>
        <input type="checkbox" class="switch" bind:checked={settings.globalShortcuts} />
      </label>
    </div>
    {#if settings.globalShortcuts}
      <p class="note keys">
        <span><kbd>Ctrl</kbd><kbd>Alt</kbd><kbd>P</kbd> 播放 / 暂停</span>
        <span><kbd>Ctrl</kbd><kbd>Alt</kbd><kbd>←</kbd><kbd>→</kbd> 上一首 / 下一首</span>
        <span><kbd>Ctrl</kbd><kbd>Alt</kbd><kbd>L</kbd> 桌面歌词</span>
      </p>
    {/if}
  </section>
{/if}

{#if show('general')}
  <section>
    <h3>通用</h3>
    <div class="group">
      <div class="item">
        <span>外观</span>
        <Select bind:value={settings.theme} options={THEMES} label="外观" />
      </div>
      <div class="item">
        <span>每次搜索数量</span>
        <Select bind:value={settings.searchLimit} options={LIMITS} label="每次搜索数量" />
      </div>
      <div class="item">
        <span>搜索记录</span>
        <button class="btn plain" onclick={() => (clearHistory(), toast.info('已清除搜索记录'))}>清除</button>
      </div>
      <div class="item">
        <span>最近播放</span>
        <button class="btn plain" onclick={() => (clearRecent(), toast.info('已清空最近播放'))}>清除</button>
      </div>
      {#if !isTauri && token.get()}
        <div class="item">
          <span>访问口令</span>
          <button class="btn plain" onclick={() => (token.set(''), location.reload())}>退出</button>
        </div>
      {/if}
    </div>
  </section>
  {#if pane === 'all'}
    <p class="foot">拾音 0.1.0 · 基于 ML-Netease_url · GPL-3.0</p>
  {/if}
{/if}

<style>
  section + section {
    margin-top: 22px;
  }
  h3 {
    margin: 0 0 6px 12px;
    font-size: var(--fs-caption);
    font-weight: 600;
    color: var(--fg-2);
    letter-spacing: 0.01em;
  }
  .group {
    border-radius: var(--r);
    background: var(--bg-2);
    overflow: hidden;
  }
  .item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    min-height: 44px;
    padding: 6px 12px 6px 14px;
    font-size: var(--fs-footnote);
    position: relative;
  }
  .item + .item::before,
  .sub-items .item::before,
  .sub-items + .item::before {
    content: '';
    position: absolute;
    left: 14px;
    right: 0;
    top: 0;
    height: 0.5px;
    background: var(--line);
  }
  .item.col {
    flex-direction: column;
    align-items: stretch;
    padding: 12px 14px;
  }
  .item.sub {
    min-height: 32px;
  }
  .sub-items {
    padding-left: 16px;
    transition: opacity var(--t) var(--ease);
  }
  .sub-items.disabled {
    opacity: 0.4;
  }
  .note {
    margin: 6px 12px 0;
    font-size: var(--fs-caption);
    color: var(--fg-3);
  }
  .switch {
    appearance: none;
    width: 38px;
    height: 22px;
    border-radius: var(--r-pill);
    background: var(--line-2);
    position: relative;
    flex: none;
    margin: 0;
    transition: background-color var(--t) var(--ease);
  }
  .switch::after {
    content: '';
    position: absolute;
    top: 2px;
    left: 2px;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: var(--white);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.25);
    transition: transform var(--t) var(--ease);
  }
  .switch:checked {
    background: var(--ok);
  }
  .switch:checked::after {
    transform: translateX(16px);
  }
  .switch:active::after {
    width: 22px;
  }
  .switch:checked:active::after {
    transform: translateX(12px);
  }
  .stepper {
    display: flex;
    align-items: center;
    height: 28px;
    border-radius: var(--r-sm);
    background: var(--bg-elev);
    box-shadow: var(--shadow-sm);
  }
  .stepper button {
    width: 28px;
    height: 28px;
    font-size: var(--fs-callout);
    color: var(--fg-2);
  }
  .stepper button:hover {
    color: var(--fg);
  }
  .stepper span {
    min-width: 20px;
    text-align: center;
    font-weight: 500;
  }
  .path {
    display: flex;
    align-items: center;
    gap: 6px;
    max-width: 60%;
    height: 28px;
    padding: 0 10px;
    border-radius: var(--r-sm);
    background: var(--bg-elev);
    box-shadow: var(--shadow-sm);
    font-size: var(--fs-caption);
  }
  .path :global(svg) {
    width: 14px;
    height: 14px;
    flex: none;
    color: var(--fg-2);
  }
  .mono {
    font-family: var(--mono);
    font-size: var(--fs-caption);
  }
  .input.mono {
    background: var(--bg-elev);
  }
  .tokens {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 10px;
  }
  .token {
    height: 24px;
    padding: 0 9px;
    border-radius: var(--r-sm);
    background: var(--accent-soft);
    color: var(--accent);
    font-size: var(--fs-caption);
    font-weight: 500;
    transition: transform var(--t-fast) var(--ease);
  }
  .token:active {
    transform: scale(0.95);
  }
  .token.ghost {
    background: var(--bg-elev);
    color: var(--fg-2);
    box-shadow: var(--shadow-sm);
    font-weight: 400;
  }
  .token.ghost.on {
    color: var(--fg);
    box-shadow: 0 0 0 1.5px var(--accent);
  }
  .sep {
    flex-basis: 100%;
    height: 0;
  }
  .preview {
    margin-top: 10px;
    font-size: var(--fs-caption);
    color: var(--fg-2);
  }
  .preview span {
    color: var(--fg);
  }
  .keys {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  kbd {
    display: inline-block;
    min-width: 18px;
    margin-right: 3px;
    padding: 0 4px;
    border-radius: var(--r-xs);
    border: 0.5px solid var(--line-2);
    background: var(--bg-elev);
    font: 500 var(--fs-caption2)/16px var(--font);
    text-align: center;
    color: var(--fg-2);
  }
  .foot {
    margin: 28px 0 0;
    text-align: center;
    font-size: var(--fs-caption2);
    color: var(--fg-3);
  }

  /* ---- 移动端：iOS 分组列表规格 ---- */
  @media (max-width: 640px) {
    section + section {
      margin-top: 28px;
    }
    h3 {
      margin: 0 0 8px 16px;
      font-size: var(--fs-footnote);
      font-weight: 400;
      text-transform: none;
      color: var(--fg-2);
    }
    .group {
      border-radius: var(--r-lg);
    }
    .item {
      min-height: 48px;
      padding: 8px 14px 8px 16px;
      font-size: var(--fs-callout);
    }
    .item + .item::before,
    .sub-items .item::before,
    .sub-items + .item::before {
      left: 16px;
    }
    .sub-items {
      padding-left: 18px;
    }
    .note {
      margin: 8px 16px 0;
      font-size: var(--fs-footnote);
    }
    .switch {
      width: 51px;
      height: 31px;
    }
    .switch::after {
      width: 27px;
      height: 27px;
    }
    .switch:checked::after {
      transform: translateX(20px);
    }
    .switch:active::after {
      width: 31px;
    }
    .switch:checked:active::after {
      transform: translateX(16px);
    }
    .stepper {
      height: 32px;
    }
    .stepper button {
      width: 36px;
      height: 32px;
      font-size: var(--fs-title3);
    }
    .path {
      height: 32px;
      font-size: var(--fs-footnote);
    }
    .token {
      height: 30px;
      padding: 0 12px;
      font-size: var(--fs-footnote);
    }
  }
</style>
