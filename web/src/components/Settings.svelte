<script lang="ts">
  import Sheet from './Sheet.svelte';
  import Icon from './Icon.svelte';
  import { app } from '../lib/app.svelte';
  import { settings } from '../lib/settings.svelte';
  import { player } from '../lib/player.svelte';
  import { clearHistory } from '../lib/history.svelte';
  import { api, isTauri, token } from '../lib/transport';
  import { toast } from '../lib/toast.svelte';
  import { LEVELS } from '../lib/types';

  let defaultDir = $state('');
  $effect(() => {
    if (isTauri && app.sheet === 'settings') api.defaultDir?.().then((d) => (defaultDir = d));
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

<Sheet open={app.sheet === 'settings'} onclose={() => (app.sheet = null)} title="设置" width={560}>
  <section>
    <h3>音质</h3>
    <div class="group">
      <label class="item">
        <span>播放音质</span>
        <select bind:value={settings.playLevel}>
          {#each LEVELS as l}<option value={l.value}>{l.label}</option>{/each}
        </select>
      </label>
      <label class="item">
        <span>下载音质</span>
        <select bind:value={settings.level}>
          {#each LEVELS as l}<option value={l.value}>{l.label}</option>{/each}
        </select>
      </label>
    </div>
    <p class="note">{LEVELS.find((l) => l.value === settings.level)?.hint}。高于账号权限时会自动降级。</p>
  </section>

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
  </section>

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

  <section>
    <h3>通用</h3>
    <div class="group">
      <label class="item">
        <span>外观</span>
        <select bind:value={settings.theme}>
          <option value="system">跟随系统</option>
          <option value="light">浅色</option>
          <option value="dark">深色</option>
        </select>
      </label>
      <label class="item">
        <span>每次搜索数量</span>
        <select bind:value={settings.searchLimit}>
          {#each [20, 30, 50, 100] as n}<option value={n}>{n}</option>{/each}
        </select>
      </label>
      <div class="item">
        <span>搜索记录</span>
        <button class="btn plain" onclick={() => (clearHistory(), toast.info('已清除搜索记录'))}>清除</button>
      </div>
      {#if !isTauri && token.get()}
        <div class="item">
          <span>访问口令</span>
          <button class="btn plain" onclick={() => (token.set(''), location.reload())}>退出</button>
        </div>
      {/if}
    </div>
  </section>

  <p class="foot">拾音 0.1.0 · 基于 ML-Netease_url · GPL-3.0</p>
</Sheet>

<style>
  section {
    margin-top: 18px;
  }
  section:first-child {
    margin-top: 0;
  }
  h3 {
    margin: 0 0 6px 12px;
    font-size: 12px;
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
    font-size: 13px;
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
    font-size: 12px;
    color: var(--fg-3);
  }
  select {
    appearance: none;
    height: 28px;
    padding: 0 28px 0 10px;
    border-radius: 7px;
    border: 0;
    background: var(--bg-elev)
      url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='%23888' stroke-width='2.4' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpath d='m7 10 5 5 5-5'/%3E%3C/svg%3E")
      no-repeat right 8px center / 12px;
    box-shadow: var(--shadow-sm);
    font-size: 13px;
    outline: none;
  }
  .switch {
    appearance: none;
    width: 38px;
    height: 22px;
    border-radius: 99px;
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
    background: #fff;
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
    border-radius: 7px;
    background: var(--bg-elev);
    box-shadow: var(--shadow-sm);
  }
  .stepper button {
    width: 28px;
    height: 28px;
    font-size: 15px;
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
    border-radius: 7px;
    background: var(--bg-elev);
    box-shadow: var(--shadow-sm);
    font-size: 12px;
  }
  .path :global(svg) {
    width: 14px;
    height: 14px;
    flex: none;
    color: var(--fg-2);
  }
  .mono {
    font-family: var(--mono);
    font-size: 12px;
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
    border-radius: 6px;
    background: var(--accent-soft);
    color: var(--accent);
    font-size: 12px;
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
    font-size: 12px;
    color: var(--fg-2);
  }
  .preview span {
    color: var(--fg);
  }
  .foot {
    margin: 22px 0 0;
    text-align: center;
    font-size: 11px;
    color: var(--fg-3);
  }
</style>
