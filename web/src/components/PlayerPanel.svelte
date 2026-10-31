<script lang="ts">
  import { fade } from '../lib/fade';
  import Icon from './Icon.svelte';
  import { player } from '../lib/player.svelte';
  import { settings } from '../lib/settings.svelte';
  import { api } from '../lib/transport';
  import { toast } from '../lib/toast.svelte';
  import { artists, duration, img, sourceName } from '../lib/format';
  import { LEVELS, type MatchCandidate, type Track } from '../lib/types';

  // 全屏播放页内部面板：与播放页同色系、同层级，不依赖全局 Sheet。
  let {
    open,
    tab = $bindable<'queue' | 'source'>('queue'),
    onclose,
  }: {
    open: boolean;
    tab?: 'queue' | 'source';
    onclose: () => void;
  } = $props();

  const t = $derived(player.current);

  // ---- 队列 ----
  let box = $state<HTMLElement>();
  $effect(() => {
    if (!open || tab !== 'queue') return;
    requestAnimationFrame(() => box?.querySelector('.cur')?.scrollIntoView({ block: 'center' }));
  });

  // 桌面拖拽排序
  let dragFrom = $state(-1);
  let dragOver = $state(-1);
  function onDragStart(e: DragEvent, i: number) {
    dragFrom = i;
    e.dataTransfer?.setData('text/plain', String(i));
    if (e.dataTransfer) e.dataTransfer.effectAllowed = 'move';
  }
  function onDragOver(e: DragEvent, i: number) {
    e.preventDefault();
    dragOver = i;
  }
  function onDrop(i: number) {
    if (dragFrom >= 0) player.move(dragFrom, i);
    dragFrom = dragOver = -1;
  }

  // ---- 换源 ----
  let candidates = $state<MatchCandidate[] | null>(null);
  let loadingCand = $state(false);
  let candFor = '';
  $effect(() => {
    if (!open || tab !== 'source' || !t) return;
    const key = `${t.source}:${t.id}:${settings.playLevel}`;
    if (candFor === key) return;
    candFor = key;
    candidates = null;
    loadingCand = true;
    api
      .matchCandidates(t.name, artists(t), t.source, settings.playLevel, t.duration)
      .then((l) => (candidates = l))
      .catch(() => (candidates = []))
      .finally(() => (loadingCand = false));
  });

  async function pick(c: MatchCandidate) {
    await player.switchSource(c);
    toast.success(`已切换到 ${sourceName(c.track.source)}`);
    tab = 'queue';
  }

  function setLevel(v: string) {
    if (settings.playLevel === v) return;
    settings.playLevel = v;
    candFor = '';
  }

  const levelLabel = (v: string) => LEVELS.find((l) => l.value === v)?.label ?? v;
  const sourceLevels = $derived(t ? (player.current ? LEVELS : []) : []);
  const sameTrack = (a: Track, b: Track | null) => !!b && a.id === b.id && a.source === b.source;
</script>

{#if open}
  <div class="scrim" onclick={onclose} role="presentation"></div>
  <aside class="panel" aria-label="播放面板">
    <header>
      <div class="seg" role="tablist">
        <button class:on={tab === 'queue'} onclick={() => (tab = 'queue')} role="tab" aria-selected={tab === 'queue'}>播放列表</button>
        <button class:on={tab === 'source'} onclick={() => (tab = 'source')} role="tab" aria-selected={tab === 'source'}>音源与音质</button>
      </div>
      <button class="icon-btn" onclick={onclose} aria-label="关闭"><Icon name="close" /></button>
    </header>

    {#if tab === 'queue'}
      <div class="body" bind:this={box}>
        {#if !player.queue.length}
          <div class="empty">播放列表是空的</div>
        {:else}
          <div class="meta-row">
            <span>{player.queue.length} 首</span>
            <button class="link" onclick={() => player.clear()}>清空</button>
          </div>
          {#each player.queue as q, i (q.source + ':' + q.id)}
            <div
              class="row"
              class:cur={i === player.index}
              class:over={dragOver === i && dragFrom !== i}
              draggable="true"
              ondragstart={(e) => onDragStart(e, i)}
              ondragover={(e) => onDragOver(e, i)}
              ondragleave={() => (dragOver = -1)}
              ondrop={() => onDrop(i)}
              ondragend={() => (dragFrom = dragOver = -1)}
              role="listitem"
            >
              <button class="main" onclick={() => player.playAt(i)}>
                <img use:fade src={img(q.cover, 40)} alt="" width="40" height="40" loading="lazy" decoding="async" />
                <span class="txt">
                  <span class="name ellipsis">{q.name}</span>
                  <span class="sub ellipsis">{artists(q)} · {sourceName(q.source)}</span>
                </span>
                <span class="dur tabular">{duration(q.duration)}</span>
              </button>
              <div class="ops">
                <button class="icon-btn mv" disabled={i === 0} onclick={() => player.move(i, i - 1)} aria-label="上移"><Icon name="chevronDown" /></button>
                <button class="icon-btn mv dn" disabled={i === player.queue.length - 1} onclick={() => player.move(i, i + 1)} aria-label="下移"><Icon name="chevronDown" /></button>
                <button class="icon-btn" onclick={() => player.remove(i)} aria-label="移除"><Icon name="close" /></button>
              </div>
            </div>
          {/each}
        {/if}
      </div>
    {:else if t}
      <div class="body">
        <section>
          <h3>当前</h3>
          <div class="card now">
            <img use:fade src={img(t.cover, 48)} alt="" width="44" height="44" decoding="async" />
            <div class="txt">
              <div class="name ellipsis">{t.name}</div>
              <div class="sub ellipsis">
                {sourceName(t.source)}
                {#if player.quality} · {levelLabel(player.quality)}{/if}
                {#if player.trial} · <span class="warn">仅试听</span>{/if}
                {#if player.matchedFrom} · 原为 {sourceName(player.matchedFrom)}{/if}
              </div>
            </div>
            <button class="icon-btn" onclick={() => player.retryCurrent()} title="重新加载" aria-label="重新加载"><Icon name="retry" /></button>
          </div>
        </section>

        <section>
          <h3>播放音质</h3>
          <div class="levels">
            {#each sourceLevels as l (l.value)}
              <button class="lvl" class:on={settings.playLevel === l.value} onclick={() => setLevel(l.value)}>
                <span>{l.label}</span>
                <small>{l.hint}</small>
              </button>
            {/each}
          </div>
        </section>

        <section>
          <h3>其它音源的版本</h3>
          {#if loadingCand}
            <div class="loading"><span class="spinner"></span><span>正在各平台查找…</span></div>
          {:else if !candidates?.length}
            <div class="empty sm">没有找到可完整播放的其它版本</div>
          {:else}
            {#each candidates as c (c.track.source + ':' + c.track.id)}
              <button class="card cand" class:on={sameTrack(c.track, t)} onclick={() => pick(c)}>
                <img use:fade src={img(c.track.cover, 48)} alt="" width="44" height="44" loading="lazy" decoding="async" />
                <div class="txt">
                  <div class="name ellipsis">{c.track.name}</div>
                  <div class="sub ellipsis">{artists(c.track)}{#if c.track.album} · {c.track.album}{/if}</div>
                </div>
                <div class="tag">
                  <span class="src">{sourceName(c.track.source)}</span>
                  <small>{levelLabel(c.url.level)}{#if c.url.format} · {c.url.format.toUpperCase()}{/if} · {duration(c.track.duration)}</small>
                </div>
              </button>
            {/each}
          {/if}
        </section>
      </div>
    {/if}
  </aside>
{/if}

<style>
  .scrim {
    position: absolute;
    inset: 0;
    z-index: 5;
    background: rgb(0 0 0 / 0.35);
    animation: fade var(--t) var(--ease) both;
  }
  .panel {
    position: absolute;
    z-index: 6;
    top: 10px;
    right: 10px;
    bottom: 10px;
    width: min(400px, calc(100vw - 20px));
    display: flex;
    flex-direction: column;
    border-radius: var(--r-lg);
    background: rgb(28 28 30 / 0.78);
    -webkit-backdrop-filter: saturate(160%) blur(28px);
    backdrop-filter: saturate(160%) blur(28px);
    box-shadow: 0 0 0 0.5px rgb(255 255 255 / 0.12), 0 24px 64px rgb(0 0 0 / 0.5);
    color: #fff;
    animation: panel-in var(--t-slow) var(--ease) both;
  }
  @keyframes panel-in {
    from {
      transform: translateX(calc(100% + 20px));
    }
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 12px 10px 10px 14px;
    flex: none;
  }
  header .icon-btn {
    color: rgb(255 255 255 / 0.7);
  }
  header .icon-btn:hover {
    background: rgb(255 255 255 / 0.12);
    color: #fff;
  }
  .seg {
    display: inline-flex;
    padding: 3px;
    border-radius: 9px;
    background: rgb(255 255 255 / 0.1);
    gap: 2px;
  }
  .seg button {
    height: 28px;
    padding: 0 12px;
    border-radius: 7px;
    font-size: 13px;
    font-weight: 500;
    color: rgb(255 255 255 / 0.6);
    transition: background-color var(--t-fast) var(--ease), color var(--t-fast) var(--ease);
  }
  .seg button.on {
    background: rgb(255 255 255 / 0.18);
    color: #fff;
  }
  .body {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 0 10px 12px;
    overscroll-behavior: contain;
  }
  .body::-webkit-scrollbar-thumb {
    background: rgb(255 255 255 / 0.2);
  }
  .empty {
    padding: 48px 0;
    text-align: center;
    color: rgb(255 255 255 / 0.45);
    font-size: 14px;
  }
  .empty.sm {
    padding: 18px 0;
    font-size: 13px;
  }
  .meta-row {
    display: flex;
    justify-content: space-between;
    padding: 2px 6px 8px;
    font-size: 12px;
    color: rgb(255 255 255 / 0.5);
  }
  .link {
    color: rgb(255 255 255 / 0.75);
    font-weight: 500;
  }
  .link:hover {
    color: #fff;
  }

  .row {
    display: flex;
    align-items: center;
    border-radius: var(--r);
    transition: background-color var(--t-fast) var(--ease);
  }
  .row:hover {
    background: rgb(255 255 255 / 0.07);
  }
  .row.cur {
    background: rgb(255 255 255 / 0.12);
  }
  .row.over {
    box-shadow: inset 0 2px 0 var(--accent);
  }
  .main {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 10px;
    height: 54px;
    padding: 0 8px;
    text-align: left;
  }
  .main img,
  .card img {
    border-radius: 6px;
    background: rgb(255 255 255 / 0.1);
    flex: none;
    object-fit: cover;
  }
  .txt {
    min-width: 0;
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .name {
    font-size: 13.5px;
    font-weight: 500;
  }
  .row.cur .name {
    color: #fff;
  }
  .sub {
    font-size: 12px;
    color: rgb(255 255 255 / 0.55);
  }
  .dur {
    font-size: 12px;
    color: rgb(255 255 255 / 0.4);
  }
  .ops {
    display: flex;
    gap: 0;
    opacity: 0;
    margin-right: 4px;
    transition: opacity var(--t-fast) var(--ease);
  }
  .row:hover .ops,
  .row:focus-within .ops {
    opacity: 1;
  }
  .ops .icon-btn {
    width: 28px;
    height: 28px;
    color: rgb(255 255 255 / 0.6);
  }
  .ops .icon-btn:hover {
    background: rgb(255 255 255 / 0.14);
    color: #fff;
  }
  .ops .icon-btn:disabled {
    opacity: 0.25;
    pointer-events: none;
  }
  .ops .icon-btn :global(svg) {
    width: 15px;
    height: 15px;
  }
  .mv :global(svg) {
    transform: rotate(180deg);
  }
  .mv.dn :global(svg) {
    transform: none;
  }

  section {
    margin-top: 10px;
    padding: 0 4px;
  }
  h3 {
    margin: 0 0 8px 6px;
    font-size: 11.5px;
    font-weight: 600;
    letter-spacing: 0.03em;
    color: rgb(255 255 255 / 0.5);
    text-transform: uppercase;
  }
  .card {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    border-radius: var(--r);
    background: rgb(255 255 255 / 0.07);
    text-align: left;
    transition: background-color var(--t-fast) var(--ease), transform var(--t-fast) var(--ease);
  }
  .card + .card {
    margin-top: 6px;
  }
  .card.cand:hover {
    background: rgb(255 255 255 / 0.12);
  }
  .card.cand:active {
    transform: scale(0.985);
  }
  .card.cand.on {
    box-shadow: inset 0 0 0 1.5px rgb(255 255 255 / 0.6);
  }
  .card .icon-btn {
    color: rgb(255 255 255 / 0.6);
  }
  .card .icon-btn:hover {
    background: rgb(255 255 255 / 0.14);
    color: #fff;
  }
  .warn {
    color: var(--warn);
  }
  .tag {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 2px;
    flex: none;
  }
  .src {
    font-size: 11px;
    font-weight: 600;
    padding: 2px 7px;
    border-radius: 5px;
    background: rgb(255 255 255 / 0.16);
  }
  .tag small {
    font-size: 10.5px;
    color: rgb(255 255 255 / 0.45);
    white-space: nowrap;
  }
  .levels {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 6px;
  }
  .lvl {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 1px;
    padding: 8px 10px;
    border-radius: var(--r);
    background: rgb(255 255 255 / 0.07);
    text-align: left;
    transition: background-color var(--t-fast) var(--ease), box-shadow var(--t-fast) var(--ease);
  }
  .lvl span {
    font-size: 13px;
    font-weight: 500;
  }
  .lvl small {
    font-size: 10.5px;
    color: rgb(255 255 255 / 0.45);
  }
  .lvl:hover {
    background: rgb(255 255 255 / 0.12);
  }
  .lvl.on {
    background: rgb(255 255 255 / 0.18);
    box-shadow: inset 0 0 0 1.5px rgb(255 255 255 / 0.5);
  }
  .loading {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 16px 8px;
    font-size: 13px;
    color: rgb(255 255 255 / 0.55);
  }
  .loading .spinner {
    border-color: rgb(255 255 255 / 0.2);
    border-top-color: #fff;
  }

  @media (max-width: 860px) {
    .panel {
      top: auto;
      left: 0;
      right: 0;
      bottom: 0;
      width: 100%;
      max-height: 78dvh;
      border-radius: 20px 20px 0 0;
      padding-bottom: var(--sab);
      animation: panel-up var(--t-sheet) var(--ease) both;
    }
    @keyframes panel-up {
      from {
        transform: translateY(100%);
      }
    }
    header {
      padding: 14px 10px 10px 14px;
    }
    .main {
      height: 58px;
    }
    .ops {
      opacity: 1;
    }
    .ops .mv {
      display: none;
    }
    .ops .icon-btn {
      width: 36px;
      height: 36px;
    }
    .levels {
      grid-template-columns: 1fr 1fr;
    }
  }
</style>
