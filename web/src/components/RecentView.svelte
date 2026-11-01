<script lang="ts">
  import Icon from './Icon.svelte';
  import TrackList from './TrackList.svelte';
  import { player } from '../lib/player.svelte';
  import { clearRecent, recent } from '../lib/recent.svelte';
  import { toast } from '../lib/toast.svelte';
  import { fade } from '../lib/fade';
  import { img } from '../lib/format';

  let { scroller }: { scroller: HTMLElement | undefined } = $props();

  const tracks = $derived(recent.items);
  /** 最近 4 首的封面拼成 2×2 封面墙 */
  const mosaic = $derived(tracks.slice(0, 4).map((t) => t.cover));
  const total = $derived(tracks.reduce((s, t) => s + t.duration, 0));

  function hours(ms: number) {
    const m = Math.round(ms / 60000);
    return m >= 60 ? `${Math.floor(m / 60)} 小时 ${m % 60} 分钟` : `${m} 分钟`;
  }
</script>

<header class="head">
  <div class="icon" class:mosaic={mosaic.length >= 4}>
    {#if mosaic.length >= 4}
      {#each mosaic as c, i (i)}
        <img use:fade src={img(c, 120)} alt="" decoding="async" />
      {/each}
    {:else if mosaic.length}
      <img use:fade class="single" src={img(mosaic[0], 240)} alt="" decoding="async" />
    {:else}
      <Icon name="clock" size={26} />
    {/if}
  </div>
  <div class="info">
    <div class="eyebrow">播放记录</div>
    <h1>最近播放</h1>
    <div class="meta muted">
      {#if tracks.length}{tracks.length} 首{#if total} · {hours(total)}{/if}{:else}暂无记录{/if}
    </div>
    <div class="cta">
      <button class="btn primary lg" onclick={() => player.playList(tracks, 0)} disabled={!tracks.length}>
        <Icon name="play" />播放
      </button>
      <button class="btn lg" onclick={() => (player.shuffle = true, player.playList(tracks, Math.floor(Math.random() * tracks.length)))} disabled={!tracks.length}>
        <Icon name="shuffle" />随机播放
      </button>
      <button class="icon-btn" onclick={() => (clearRecent(), toast.info('已清空最近播放'))} disabled={!tracks.length} title="清空" aria-label="清空最近播放">
        <Icon name="trash" />
      </button>
    </div>
  </div>
</header>

{#if !tracks.length}
  <div class="empty muted">还没有播放记录，去发现页放一首吧</div>
{:else}
  <TrackList {tracks} {scroller} onplay={(i) => player.playList(tracks, i)} />
{/if}

<style>
  .head {
    display: flex;
    gap: 24px;
    align-items: flex-end;
    padding: 8px 8px 24px;
    animation: rise var(--t-slow) var(--ease-out) backwards;
  }
  .icon {
    width: 160px;
    height: 160px;
    flex: none;
    display: grid;
    place-items: center;
    border-radius: var(--r-lg);
    overflow: hidden;
    background: linear-gradient(150deg, var(--bg-3), var(--bg-2));
    box-shadow: var(--cover-shadow);
    color: var(--fg-2);
  }
  .icon.mosaic {
    grid-template-columns: 1fr 1fr;
    grid-template-rows: 1fr 1fr;
    gap: 0;
  }
  .icon img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .info {
    min-width: 0;
    flex: 1;
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
    font: 700 30px/1.15 var(--font-display);
    letter-spacing: -0.025em;
  }
  .meta {
    margin-top: 6px;
    font-size: 13px;
  }
  .cta {
    display: flex;
    gap: 10px;
    align-items: center;
    margin-top: 18px;
  }
  .empty {
    padding: 72px 0;
    text-align: center;
  }

  @media (max-width: 760px) {
    .head {
      flex-direction: column;
      align-items: center;
      text-align: center;
      gap: 16px;
      padding: 8px 0 20px;
    }
    .icon {
      width: min(50vw, 180px);
      height: min(50vw, 180px);
      border-radius: var(--r-xl);
    }
    h1 {
      font-size: 24px;
    }
    .cta {
      justify-content: center;
    }
    .cta .btn.lg {
      flex: 1 1 0;
      padding: 0 12px;
      height: 42px;
    }
    .cta .icon-btn {
      width: 42px;
      height: 42px;
      background: var(--bg-2);
    }
  }
</style>
