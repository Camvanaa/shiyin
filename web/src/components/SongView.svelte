<script lang="ts">
  import { fade } from '../lib/fade';
  import Icon from './Icon.svelte';
  import { app } from '../lib/app.svelte';
  import { player } from '../lib/player.svelte';
  import { api } from '../lib/transport';
  import { settings } from '../lib/settings.svelte';
  import { artists, bytes, duration, img } from '../lib/format';
  import { parseLyric } from '../lib/lrc';
  import { LEVELS, type Track } from '../lib/types';

  let { track }: { track: Track } = $props();

  const lyric = $derived(api.lyric(track.id, track.source).then((l) => parseLyric(l.lrc, l.tlrc)));
  const info = $derived(api.songUrl(track.id, settings.level, track.source).catch(() => null));
  const levelLabel = (v: string) => LEVELS.find((l) => l.value === v)?.label ?? v;
  const isCurrent = $derived(player.current?.id === track.id);
</script>

<div class="song">
  <div class="art">
    <img use:fade src={img(track.cover, 320)} alt="" width="320" height="320" decoding="async" />
  </div>
  <div class="info">
    <div class="eyebrow">歌曲</div>
    <h1>{track.name}</h1>
    {#if track.alias}<div class="alias muted">{track.alias}</div>{/if}
    <div class="owner">{artists(track)}</div>
    <button class="album" onclick={() => app.openCollection('album', track.albumId, undefined, track.source)}>
      <Icon name="disc" />{track.album}<Icon name="chevronRight" />
    </button>

    <dl class="facts">
      <div><dt>时长</dt><dd class="tabular">{duration(track.duration)}</dd></div>
      {#await info then u}
        {#if u}
          <div><dt>下载音质</dt><dd>{levelLabel(u.level)}{#if u.format} · {u.format.toUpperCase()}{/if}</dd></div>
          <div><dt>大小</dt><dd class="tabular">{bytes(u.size)}</dd></div>
          <div><dt>码率</dt><dd class="tabular">{Math.round(u.br / 1000)} kbps</dd></div>
        {:else}
          <div><dt>下载音质</dt><dd class="muted">不可用，可能需要登录或会员</dd></div>
        {/if}
      {/await}
    </dl>

    <div class="cta">
      <button class="btn primary lg" onclick={() => player.play(track)}>
        <Icon name={isCurrent && player.playing ? 'pause' : 'play'} />{isCurrent && player.playing ? '暂停' : '播放'}
      </button>
      <button class="btn lg" onclick={() => app.download([track], track.name, { subtitle: artists(track), cover: track.cover })}>
        <Icon name="download" />下载
      </button>
      <button class="icon-btn" onclick={() => player.playNext(track)} title="下一首播放" aria-label="下一首播放">
        <Icon name="queue" />
      </button>
      <button
        class="icon-btn"
        onclick={() => navigator.clipboard?.writeText(`https://music.163.com/song?id=${track.id}`)}
        title="复制链接"
        aria-label="复制链接"><Icon name="link" /></button
      >
    </div>
  </div>
</div>

<section class="lyric">
  <h2>歌词</h2>
  {#await lyric}
    <div class="spinner"></div>
  {:then lines}
    {#if lines.length}
      {#each lines as l}
        <p>{l.text}{#if l.trans}<span>{l.trans}</span>{/if}</p>
      {/each}
    {:else}
      <p class="muted">暂无歌词</p>
    {/if}
  {:catch}
    <p class="muted">歌词加载失败</p>
  {/await}
</section>

<style>
  .song {
    display: flex;
    gap: 40px;
    align-items: center;
    padding: 16px 8px 32px;
    animation: rise var(--t-slow) var(--ease-out) backwards;
  }
  .art {
    width: 280px;
    height: 280px;
    flex: none;
    border-radius: var(--r-lg);
    overflow: hidden;
    background: var(--bg-3);
    box-shadow: var(--cover-shadow);
  }
  .art img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .info {
    min-width: 0;
  }
  .eyebrow {
    font-size: 12px;
    font-weight: 600;
    color: var(--fg-2);
    letter-spacing: 0.02em;
    margin-bottom: 6px;
  }
  h1 {
    margin: 0;
    font: 700 32px/1.15 var(--font-display);
    letter-spacing: -0.025em;
  }
  .alias {
    margin-top: 2px;
    font-size: 15px;
  }
  .owner {
    margin-top: 6px;
    font-size: 19px;
    font-weight: 500;
    color: var(--accent);
    letter-spacing: -0.015em;
  }
  .album {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    margin-top: 8px;
    font-size: 14px;
    color: var(--fg-2);
    border-radius: var(--r-sm);
    transition: color var(--t-fast) var(--ease);
  }
  .album:hover {
    color: var(--fg);
  }
  .album :global(svg) {
    width: 15px;
    height: 15px;
  }
  .facts {
    display: flex;
    gap: 28px;
    margin: 22px 0 0;
    flex-wrap: wrap;
  }
  .facts div {
    min-width: 0;
  }
  dt {
    font-size: 11px;
    color: var(--fg-3);
    font-weight: 500;
    letter-spacing: 0.02em;
  }
  dd {
    margin: 2px 0 0;
    font-size: 14px;
    font-weight: 500;
  }
  .cta {
    display: flex;
    gap: 10px;
    align-items: center;
    margin-top: 24px;
  }
  .lyric {
    padding: 8px;
    max-width: 640px;
    animation: rise var(--t-slow) var(--ease-out) 80ms backwards;
  }
  h2 {
    font: 600 20px var(--font-display);
    letter-spacing: -0.02em;
    margin: 0 0 14px;
  }
  .lyric p {
    margin: 0 0 12px;
    font-size: 15px;
    line-height: 1.5;
  }
  .lyric p span {
    display: block;
    font-size: 13px;
    color: var(--fg-2);
  }
  @media (max-width: 760px) {
    .song {
      flex-direction: column;
      align-items: flex-start;
      gap: 24px;
    }
    .art {
      width: 200px;
      height: 200px;
    }
  }
</style>
