<script lang="ts">
  import Icon from './Icon.svelte';
  import { app } from '../lib/app.svelte';
  import { forget, suggest, type HistoryEntry } from '../lib/history.svelte';
  import { img } from '../lib/format';

  let { large = false, autofocus = false }: { large?: boolean; autofocus?: boolean } = $props();

  let value = $state('');
  let focused = $state(false);
  let cursor = $state(-1);
  let input = $state<HTMLInputElement>();

  const items = $derived(focused ? suggest(value) : []);
  const open = $derived(focused && items.length > 0);
  const looksLikeLink = $derived(/music\.163\.com|163cn\.tv/.test(value));

  $effect(() => {
    if (autofocus) input?.focus();
  });

  function submit(q = value) {
    if (!q.trim()) return;
    value = q;
    input?.blur();
    focused = false;
    app.submit(q);
  }

  function pick(h: HistoryEntry) {
    submit(h.q);
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === 'ArrowDown' && open) {
      e.preventDefault();
      cursor = (cursor + 1) % items.length;
    } else if (e.key === 'ArrowUp' && open) {
      e.preventDefault();
      cursor = (cursor - 1 + items.length) % items.length;
    } else if (e.key === 'Enter') {
      e.preventDefault();
      if (open && cursor >= 0) pick(items[cursor]);
      else submit();
    } else if (e.key === 'Escape') {
      input?.blur();
    }
  }

  $effect(() => {
    void value;
    cursor = -1;
  });

  // 全局快捷键：⌘K / Ctrl+K / “/” 聚焦
  function onGlobalKey(e: KeyboardEvent) {
    const typing = (e.target as HTMLElement)?.closest('input, textarea, select, [contenteditable]');
    if (((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k') || (e.key === '/' && !typing)) {
      e.preventDefault();
      input?.focus();
      input?.select();
    }
  }

  // 粘贴链接后自动解析
  function onPaste(e: ClipboardEvent) {
    const text = e.clipboardData?.getData('text') ?? '';
    if (/music\.163\.com|163cn\.tv/.test(text)) {
      e.preventDefault();
      submit(text.trim());
    }
  }

  const kindIcon = { query: 'clock', song: 'note', playlist: 'list', album: 'disc' } as const;
  const kindLabel = { query: '', song: '歌曲', playlist: '歌单', album: '专辑' } as const;
</script>

<svelte:window onkeydown={onGlobalKey} />

<div class="box" class:large class:focused class:open>
  <div class="field">
    <span class="lead">
      {#if app.loading}<span class="spinner"></span>{:else}<Icon name={looksLikeLink ? 'link' : 'search'} />{/if}
    </span>
    <input
      bind:this={input}
      bind:value
      onfocus={() => (focused = true)}
      onblur={() => setTimeout(() => (focused = false), 120)}
      onkeydown={onKey}
      onpaste={onPaste}
      placeholder="搜索歌曲，或粘贴歌曲、歌单、专辑链接"
      spellcheck="false"
      autocomplete="off"
      enterkeyhint="search"
      aria-label="搜索"
    />
    {#if value}
      <button class="clear" onmousedown={(e) => e.preventDefault()} onclick={() => ((value = ''), input?.focus())} aria-label="清空">
        <Icon name="close" stroke={2.2} />
      </button>
    {:else if !large}
      <kbd>⌘K</kbd>
    {/if}
  </div>

  {#if open}
    <div class="pop" role="listbox">
      {#if !value}<div class="cap">最近</div>{/if}
      {#each items as h, i (h.at)}
        <div class="opt" class:cur={i === cursor} role="option" aria-selected={i === cursor} tabindex="-1">
          <button class="opt-main" onmousedown={(e) => e.preventDefault()} onclick={() => pick(h)}>
            {#if h.cover}
              <img src={img(h.cover, 28)} alt="" width="28" height="28" class:round={h.kind === 'query'} />
            {:else}
              <span class="ico"><Icon name={kindIcon[h.kind]} /></span>
            {/if}
            <span class="ellipsis">{h.title}</span>
            {#if kindLabel[h.kind]}<span class="kind">{kindLabel[h.kind]}</span>{/if}
          </button>
          <button class="rm" onmousedown={(e) => e.preventDefault()} onclick={() => forget(h.at)} aria-label="删除记录">
            <Icon name="close" />
          </button>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .box {
    position: relative;
    width: 100%;
  }
  .field {
    position: relative;
    display: flex;
    align-items: center;
    height: 36px;
    border-radius: var(--r);
    background: var(--bg-2);
    transition:
      background-color var(--t) var(--ease),
      box-shadow var(--t) var(--ease),
      border-radius var(--t) var(--ease);
  }
  .large .field {
    height: 52px;
    border-radius: var(--r-lg);
  }
  .focused .field {
    background: var(--bg-elev);
    box-shadow: 0 0 0 1px var(--line-2), 0 0 0 4px var(--accent-soft);
  }
  .large.focused .field {
    box-shadow: var(--shadow);
  }
  .lead {
    width: 36px;
    display: grid;
    place-items: center;
    color: var(--fg-3);
    flex: none;
  }
  .large .lead {
    width: 52px;
  }
  .lead :global(svg) {
    width: 17px;
    height: 17px;
  }
  .large .lead :global(svg) {
    width: 20px;
    height: 20px;
  }
  input {
    flex: 1;
    min-width: 0;
    height: 100%;
    border: 0;
    outline: 0;
    background: transparent;
    font-size: 14px;
  }
  .large input {
    font-size: 17px;
    letter-spacing: -0.015em;
  }
  input::placeholder {
    color: var(--fg-3);
  }
  .clear {
    width: 20px;
    height: 20px;
    margin: 0 10px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    background: var(--fg-3);
    color: var(--bg);
    flex: none;
    animation: fade var(--t-fast) var(--ease);
  }
  .clear :global(svg) {
    width: 10px;
    height: 10px;
  }
  kbd {
    margin-right: 10px;
    font: 500 11px var(--font);
    color: var(--fg-3);
    border: 1px solid var(--line-2);
    border-radius: 5px;
    padding: 1px 5px;
  }

  .pop {
    position: absolute;
    left: 0;
    right: 0;
    top: calc(100% + 6px);
    z-index: 40;
    padding: 6px;
    border-radius: var(--r-lg);
    background: var(--bg-elev);
    box-shadow: var(--shadow);
    animation: pop-in var(--t) var(--ease) both;
    transform-origin: top center;
  }
  @keyframes pop-in {
    from {
      opacity: 0;
      transform: translateY(-4px) scale(0.985);
    }
  }
  .cap {
    padding: 6px 10px 4px;
    font-size: 11px;
    font-weight: 600;
    color: var(--fg-3);
    letter-spacing: 0.02em;
  }
  .opt {
    display: flex;
    align-items: center;
    border-radius: var(--r-sm);
  }
  .opt:hover,
  .opt.cur {
    background: var(--hover);
  }
  .opt-main {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 10px;
    height: 40px;
    padding: 0 10px;
    text-align: left;
    font-size: 14px;
  }
  .opt-main img {
    border-radius: 4px;
    flex: none;
    object-fit: cover;
  }
  .ico {
    width: 28px;
    height: 28px;
    display: grid;
    place-items: center;
    color: var(--fg-3);
    flex: none;
  }
  .ico :global(svg) {
    width: 16px;
    height: 16px;
  }
  .kind {
    margin-left: auto;
    font-size: 11px;
    color: var(--fg-3);
    flex: none;
  }
  .rm {
    width: 28px;
    height: 28px;
    margin-right: 6px;
    display: grid;
    place-items: center;
    border-radius: 50%;
    color: var(--fg-3);
    opacity: 0;
    transition: opacity var(--t-fast) var(--ease);
  }
  .opt:hover .rm {
    opacity: 1;
  }
  .rm:hover {
    background: var(--press);
    color: var(--fg);
  }
  .rm :global(svg) {
    width: 13px;
    height: 13px;
  }
</style>
