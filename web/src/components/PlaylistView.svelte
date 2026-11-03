<script lang="ts">
  import Icon from './Icon.svelte';
  import TrackList from './TrackList.svelte';
  import { app } from '../lib/app.svelte';
  import { player } from '../lib/player.svelte';
  import { library } from '../lib/library.svelte';
  import { toast } from '../lib/toast.svelte';
  import { fade } from '../lib/fade';
  import { img } from '../lib/format';
  import type { LibPlaylist, LibTrack, Track } from '../lib/types';

  let { data, scroller }: { data: LibPlaylist; scroller: HTMLElement | undefined } = $props();

  // 以 store 里的版本为准（移除曲目后要刷新）
  const pl = $derived(library.playlists.find((p) => p.id === data.id) ?? data);

  function toTrack(t: LibTrack): Track {
    return {
      id: t.id,
      source: t.source,
      name: t.name,
      alias: null,
      artists: t.artists,
      albumId: t.albumId,
      album: t.album,
      cover: t.cover,
      duration: t.duration,
      fee: 0,
      availability: 'ok',
      isrc: t.isrc ?? null,
      alts: t.alts.map((k) => {
        const [source, ...rest] = k.split(':');
        return { source, id: rest.join(':') };
      }),
    };
  }

  const tracks = $derived(pl.tracks.map(toTrack));

  async function rename() {
    const name = prompt('歌单名称', pl.name);
    if (!name?.trim() || name === pl.name) return;
    await library.renamePlaylist(pl.id, name.trim());
  }

  async function remove() {
    if (!confirm(`删除歌单「${pl.name}」？`)) return;
    await library.deletePlaylist(pl.id);
    toast.info('已删除歌单');
    app.openLibrary();
  }

  async function removeAt(i: number) {
    const t = pl.tracks[i];
    await library.removeFromPlaylist(pl.id, `${t.source}:${t.id}`);
  }
</script>

<header class="head">
  <div class="art">
    {#if pl.cover}
      <img use:fade src={img(pl.cover, 240)} alt="" width="240" height="240" decoding="async" />
    {:else}
      <span class="ph"><Icon name="note" size={28} /></span>
    {/if}
  </div>
  <div class="info">
    <div class="eyebrow">歌单</div>
    <h1>{pl.name}</h1>
    <div class="meta muted">{tracks.length} 首</div>
    <div class="cta">
      <button class="btn primary lg" onclick={() => player.playList(tracks, 0)} disabled={!tracks.length}><Icon name="play" />播放</button>
      <button class="btn lg" onclick={() => (player.shuffle = true, player.playList(tracks, Math.floor(Math.random() * tracks.length)))} disabled={!tracks.length}>
        <Icon name="shuffle" />随机播放
      </button>
      <button class="icon-btn" onclick={rename} title="重命名" aria-label="重命名"><Icon name="list" /></button>
      <button class="icon-btn" onclick={remove} title="删除歌单" aria-label="删除歌单"><Icon name="trash" /></button>
    </div>
  </div>
</header>

{#if !tracks.length}
  <div class="blank">
    <Icon name="note" size={28} />
    <div class="blank-title">歌单是空的</div>
    <div class="blank-sub">在歌曲右键菜单里选择「加入歌单」</div>
  </div>
{:else}
  <TrackList {tracks} {scroller} onplay={(i) => player.playList(tracks, i)} />
  <div class="hint muted">在歌曲右键菜单里可移除</div>
{/if}

<style>
  .head {
    display: flex;
    gap: 24px;
    align-items: flex-end;
    padding: 8px 8px 24px;
    animation: rise var(--t-slow) var(--ease-out) backwards;
  }
  .art {
    width: 180px;
    height: 180px;
    flex: none;
    display: grid;
    place-items: center;
    border-radius: var(--r-lg);
    overflow: hidden;
    background: var(--bg-3);
    box-shadow: var(--cover-shadow);
    color: var(--fg-3);
  }
  .art img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .info {
    min-width: 0;
    flex: 1;
  }
  .eyebrow {
    font-size: var(--fs-caption);
    font-weight: 600;
    color: var(--fg-2);
    letter-spacing: 0.02em;
    margin-bottom: 6px;
  }
  h1 {
    margin: 0;
    font: 700 var(--fs-title1)/1.15 var(--font-display);
    letter-spacing: -0.025em;
  }
  .meta {
    margin-top: 6px;
    font-size: var(--fs-footnote);
  }
  .cta {
    display: flex;
    gap: 10px;
    align-items: center;
    margin-top: 18px;
  }
  .hint {
    padding: 8px 8px 40px;
    font-size: var(--fs-caption2);
  }
  .blank {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    padding: 64px 20px;
    text-align: center;
    color: var(--fg-3);
  }
  .blank :global(svg) {
    width: 32px;
    height: 32px;
    margin-bottom: 4px;
    opacity: 0.7;
  }
  .blank-title {
    font-size: var(--fs-headline);
    font-weight: 600;
    color: var(--fg);
  }
  .blank-sub {
    font-size: var(--fs-footnote);
    color: var(--fg-2);
  }

  @media (max-width: 760px) {
    .head {
      flex-direction: column;
      align-items: center;
      text-align: center;
      gap: 16px;
    }
    .art {
      width: min(48vw, 170px);
      height: min(48vw, 170px);
      border-radius: var(--r-xl);
    }
    h1 {
      font-size: var(--fs-title2);
    }
    .cta {
      justify-content: center;
    }
    .cta .btn.lg {
      flex: 1 1 0;
      padding: 0 12px;
      height: 42px;
    }
  }
</style>
