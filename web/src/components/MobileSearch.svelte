<script lang="ts">
  import Icon from './Icon.svelte';
  import { app, SOURCE_TABS } from '../lib/app.svelte';
  import { history, clearHistory, forget } from '../lib/history.svelte';
  import { img, LINK_RE } from '../lib/format';

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
  const charts = [
    { id: '3778678', name: '热歌榜', source: 'netease', kind: 'playlist', sub: '网易云' },
    { id: '26', name: '热歌榜', source: 'qq', kind: 'toplist', sub: 'QQ音乐' },
    { id: '3779629', name: '新歌榜', source: 'netease', kind: 'playlist', sub: '网易云' },
    { id: '62', name: '飙升榜', source: 'qq', kind: 'toplist', sub: 'QQ音乐' },
  ] as const;
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

  <div class="seg-ctl src" role="tablist" aria-label="音源">
    {#each SOURCE_TABS as [id, label] (id)}
      <button class:on={app.source === id} onclick={() => app.setSource(id)} role="tab" aria-selected={app.source === id}>{label}</button>
    {/each}
  </div>

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
    <div class="group two">
      {#each charts as c (c.source + c.id)}
        <button class="row" onclick={() => app.openCollection(c.kind, c.id, undefined, c.source)}>
          <span class="r-text">
            <span class="r-name">{c.name}</span>
            <span class="r-sub">{c.sub}</span>
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
    border-radius: 11px;
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
    font-size: 16px; /* ≥16px 防止 iOS 聚焦时自动放大页面 */
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

  .src {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    margin-top: 10px;
  }
  .src button {
    height: 30px;
    padding: 0;
    font-size: 13px;
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
    font: 600 17px var(--font-display);
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
    border-radius: 999px;
    background: var(--bg-2);
    font-size: 13px;
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
    width: 30px;
    height: 100%;
    display: grid;
    place-items: center;
    color: var(--fg-3);
  }

  .group {
    border-radius: var(--r-lg);
    background: var(--bg-2);
    overflow: hidden;
  }
  .group.two {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
    background: none;
    border-radius: 0;
  }
  .group.two .row {
    border-radius: var(--r-lg);
    background: var(--bg-2);
    height: 60px;
    padding: 0 12px 0 16px;
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
    width: 44px;
    height: 44px;
    border-radius: 8px;
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
    font-size: 15px;
    font-weight: 500;
    color: var(--fg);
    letter-spacing: -0.01em;
  }
  .r-sub {
    font-size: 12px;
    color: var(--fg-2);
  }
</style>
