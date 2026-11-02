<script lang="ts">
  import Icon from './Icon.svelte';
  import SourceFilter from './SourceFilter.svelte';
  import { app } from '../lib/app.svelte';
  import { history, clearHistory, forget } from '../lib/history.svelte';
  import { recent } from '../lib/recent.svelte';
  import { player } from '../lib/player.svelte';
  import { artists } from '../lib/format';
  import { img, LINK_RE, sourceShort } from '../lib/format';

  let value = $state('');
  let input = $state<HTMLInputElement>();

  const recentQueries = $derived(history.items.filter((h) => h.kind === 'query').slice(0, 12));
  const recentOpened = $derived(history.items.filter((h) => h.kind !== 'query').slice(0, 6));
  const isLink = $derived(LINK_RE.test(value));

  function submit(q = value) {
    const text = q.trim();
    if (!text) return;
    value = text;
    input?.blur();
    app.submit(text);
  }

  function onPaste(e: ClipboardEvent) {
    const text = e.clipboardData?.getData('text') ?? '';
    if (LINK_RE.test(text)) {
      e.preventDefault();
      submit(text);
    }
  }

  // 进入页面自动聚焦，减少一次点击
  $effect(() => {
    const id = setTimeout(() => input?.focus(), 350);
    return () => clearTimeout(id);
  });

  const kindLabel = { song: '歌曲', playlist: '歌单', album: '专辑', query: '' } as const;
  type Chart = { id: string; name: string; source: string; kind: 'playlist' | 'album' | 'toplist'; subtitle: string; cover: string };
  /** 固定入口：后端 feed 未就绪时也能进榜 */
  const pinned: Chart[] = [
    { id: '3778678', name: '热歌榜', source: 'netease', kind: 'playlist', subtitle: '网易云', cover: '' },
    { id: '26', name: '热歌榜', source: 'qq', kind: 'toplist', subtitle: 'QQ音乐', cover: '' },
    { id: '3779629', name: '新歌榜', source: 'netease', kind: 'playlist', subtitle: '网易云', cover: '' },
    { id: '62', name: '飙升榜', source: 'qq', kind: 'toplist', subtitle: 'QQ音乐', cover: '' },
  ];
  const charts = $derived.by<Chart[]>(() => {
    const live: Chart[] = (app.feed?.charts ?? []).slice(0, 8).map((c) => ({
      id: String(c.id),
      name: c.name,
      source: c.source,
      kind: c.kind || 'toplist',
      subtitle: c.subtitle || sourceShort(c.source),
      cover: c.cover,
    }));
    const extra = pinned.filter((p) => !live.some((c) => c.id === p.id && c.source === p.source));
    return live.concat(extra).slice(0, 8);
  });
</script>

<div class="ms">
  <div class="field" class:active={!!value}>
    <span class="lead">{#if app.loading}<span class="spinner"></span>{:else}<Icon name={isLink ? 'link' : 'search'} size={17} />{/if}</span>
    <input
      bind:this={input}
      bind:value
      onkeydown={(e) => e.key === 'Enter' && (e.preventDefault(), submit())}
      onpaste={onPaste}
      placeholder="歌曲、歌手，或粘贴链接"
      autocomplete="off"
      spellcheck="false"
      enterkeyhint="search"
      type="search"
      aria-label="搜索"
    />
    {#if value}
      <button class="clear" onclick={() => ((value = ''), input?.focus())} aria-label="清空">
        <Icon name="close" size={11} stroke={2.6} />
      </button>
    {/if}
  </div>

  <SourceFilter variant="chips" />

  {#if recentQueries.length}
    <section>
      <div class="sec-head">
        <h3>最近搜索</h3>
        <button class="btn plain" onclick={clearHistory}>清除</button>
      </div>
      <div class="chips">
        {#each recentQueries as q (q.at)}
          <span class="chip">
            <button class="chip-main ellipsis" onclick={() => submit(q.q)}>{q.title}</button>
            <button class="chip-rm" onclick={() => forget(q.at)} aria-label="删除"><Icon name="close" size={10} stroke={2.6} /></button>
          </span>
        {/each}
      </div>
    </section>
  {/if}

  {#if recent.items.length}
    <section>
      <button class="sec-head link" onclick={() => app.openHistory()}>
        <h3>最近播放</h3>
        <span class="more-arrow"><Icon name="chevronRight" size={17} stroke={2.2} /></span>
      </button>
      <div class="group">
        {#each recent.items.slice(0, 3) as t, i (`${t.source}:${t.id}`)}
          <button class="row" onclick={() => player.playList(recent.items, i)}>
            <span class="r-cover">
              {#if t.cover}<img src={img(t.cover, 56)} alt="" width="44" height="44" loading="lazy" decoding="async" />{/if}
            </span>
            <span class="r-text">
              <span class="r-name ellipsis">{t.name}</span>
              <span class="r-sub ellipsis">{artists(t)}</span>
            </span>
            <Icon name="play" size={16} />
          </button>
        {/each}
      </div>
    </section>
  {/if}

  {#if recentOpened.length}
    <section>
      <div class="sec-head"><h3>最近打开</h3></div>
      <div class="group">
        {#each recentOpened as h (h.at)}
          <button class="row" onclick={() => app.submit(h.q)}>
            <span class="r-cover">
              {#if h.cover}
                <img src={img(h.cover, 56)} alt="" width="44" height="44" loading="lazy" decoding="async" />
              {:else}
                <Icon name={h.kind === 'album' ? 'disc' : h.kind === 'song' ? 'note' : 'list'} size={18} />
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

  <section>
    <div class="sec-head"><h3>排行榜</h3></div>
    <div class="charts">
      {#each charts as c, i (c.source + ':' + c.id)}
        <button class="row chart" onclick={() => app.openCollection(c.kind, c.id, undefined, c.source)}>
          <span class="rank tabular">{i + 1}</span>
          <span class="r-cover">
            {#if c.cover}<img src={img(c.cover, 64)} alt="" width="44" height="44" loading="lazy" decoding="async" />{:else}<Icon name="list" size={18} />{/if}
          </span>
          <span class="r-text">
            <span class="r-name ellipsis">{c.name}</span>
            <span class="r-sub ellipsis">{c.subtitle}</span>
          </span>
          <Icon name="chevronRight" size={16} />
        </button>
      {/each}
    </div>
  </section>
</div>

<style>
  .ms {
    padding: 4px 0 24px;
  }
  .field {
    display: flex;
    align-items: center;
    height: 40px;
    padding: 0 6px 0 10px;
    border-radius: var(--r);
    background: var(--bg-2);
    color: var(--fg-3);
    transition: box-shadow var(--t) var(--ease), background-color var(--t) var(--ease);
  }
  .field:focus-within {
    background: var(--bg-elev);
    box-shadow: 0 0 0 1px var(--line-2), 0 0 0 4px var(--accent-soft);
  }
  .lead {
    width: 24px;
    display: grid;
    place-items: center;
    flex: none;
  }
  .field input {
    flex: 1;
    min-width: 0;
    height: 100%;
    margin-left: 6px;
    border: 0;
    outline: none;
    background: transparent;
    font-size: var(--fs-body); /* ≥16px 防止 iOS 聚焦时自动放大页面 */
    color: var(--fg);
    appearance: none;
  }
  .field input::-webkit-search-cancel-button {
    display: none;
  }
  .field input::placeholder {
    color: var(--fg-3);
  }
  .clear {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    color: var(--bg);
    flex: none;
  }
  .clear::before {
    content: '';
    position: absolute;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: var(--fg-3);
  }
  .clear :global(svg) {
    position: relative;
  }

  section {
    margin-top: 26px;
  }
  .sec-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    margin-bottom: 10px;
    padding: 0 2px;
  }
  h3 {
    margin: 0;
    font: 600 var(--fs-headline) var(--font-display);
    letter-spacing: -0.015em;
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    height: 32px;
    padding-left: 13px;
    border-radius: var(--r-pill);
    background: var(--bg-2);
    font-size: var(--fs-footnote);
    max-width: 100%;
    transition: transform var(--t-fast) var(--ease);
  }
  .chip:active {
    transform: scale(0.96);
  }
  .chip-main {
    max-width: 200px;
    height: 100%;
    color: var(--fg);
  }
  .chip-rm {
    width: 44px;
    height: 100%;
    display: grid;
    place-items: center;
    color: var(--fg-3);
  }

  .group,
  .charts {
    border-radius: var(--r-lg);
    background: var(--bg-2);
    overflow: hidden;
  }
  .rank {
    width: 16px;
    flex: none;
    font-size: var(--fs-footnote);
    font-weight: 600;
    color: var(--fg-3);
    text-align: center;
  }
  .chart:nth-child(-n + 3) .rank {
    color: var(--fg);
  }
  .charts .row {
    gap: 10px;
    padding-left: 8px;
  }
  .charts .row + .row::before {
    content: '';
    position: absolute;
    left: 78px;
    right: 0;
    top: 0;
    height: 0.5px;
    background: var(--line);
  }
  .sec-head.link {
    width: 100%;
    text-align: left;
    align-items: center;
    justify-content: flex-start;
    gap: 2px;
    color: inherit;
  }
  .more-arrow {
    display: inline-grid;
    place-items: center;
    color: var(--fg-3);
    transition: transform var(--t) var(--ease);
  }
  .sec-head.link:active .more-arrow {
    transform: translateX(2px);
  }
  .row {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 12px;
    height: 60px;
    padding: 0 12px 0 10px;
    text-align: left;
    color: var(--fg-3);
    position: relative;
    transition: background-color var(--t-fast) var(--ease);
  }
  .row:active {
    background: var(--press);
  }
  .group:not(.two) .row + .row::before {
    content: '';
    position: absolute;
    left: 66px;
    right: 0;
    top: 0;
    height: 0.5px;
    background: var(--line);
  }
  .r-cover {
    position: relative;
    width: 44px;
    height: 44px;
    border-radius: var(--r-md);
    overflow: hidden;
    background: var(--bg-3);
    display: grid;
    place-items: center;
    flex: none;
    color: var(--fg-3);
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
    gap: 1px;
  }
  .r-name {
    font-size: var(--fs-callout);
    font-weight: 500;
    color: var(--fg);
    letter-spacing: -0.01em;
  }
  .r-sub {
    font-size: var(--fs-caption);
    color: var(--fg-2);
  }
</style>
