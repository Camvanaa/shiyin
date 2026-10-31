<script lang="ts">
  import { fade } from '../lib/fade';
  import Icon from './Icon.svelte';
  import Sheet from './Sheet.svelte';
  import { app } from '../lib/app.svelte';
  import { player } from '../lib/player.svelte';
  import { artists, duration, img } from '../lib/format';

  let box = $state<HTMLElement>();

  // 打开时把当前歌曲滚动到可见位置
  $effect(() => {
    if (app.sheet !== 'queue') return;
    requestAnimationFrame(() => {
      box?.querySelector('.cur')?.scrollIntoView({ block: 'center' });
    });
  });
</script>

<Sheet open={app.sheet === 'queue'} onclose={() => (app.sheet = null)} title="播放列表" side width={380}>
  {#snippet actions()}
    {#if player.queue.length}
      <button class="btn plain" onclick={() => player.clear()}>清空</button>
    {/if}
  {/snippet}
  {#if !player.queue.length}
    <div class="empty muted">播放列表是空的</div>
  {:else}
    <div class="count muted">{player.queue.length} 首</div>
    <div bind:this={box}>
      {#each player.queue as t, i (t.source + ':' + t.id)}
        <div class="row" class:cur={i === player.index}>
          <button class="main" onclick={() => player.playAt(i)}>
            <img use:fade src={img(t.cover, 36)} alt="" width="36" height="36" loading="lazy" decoding="async" />
            <span class="txt">
              <span class="name ellipsis">{t.name}</span>
              <span class="sub ellipsis">{artists(t)}</span>
            </span>
            <span class="dur tabular">{duration(t.duration)}</span>
          </button>
          <button class="icon-btn rm" onclick={() => player.remove(i)} aria-label="移除"><Icon name="close" /></button>
        </div>
      {/each}
    </div>
  {/if}
</Sheet>

<style>
  .empty {
    padding: 64px 0;
    text-align: center;
  }
  .count {
    font-size: 12px;
    margin-bottom: 6px;
  }
  .row {
    display: flex;
    align-items: center;
    border-radius: var(--r);
    margin: 0 -8px;
    transition: background-color var(--t-fast) var(--ease);
  }
  .row:hover {
    background: var(--hover);
  }
  .row.cur {
    background: var(--bg-2);
  }
  .row.cur .name {
    color: var(--accent);
  }
  .main {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 10px;
    height: 52px;
    padding: 0 8px;
    text-align: left;
  }
  .main img {
    border-radius: 5px;
    background: var(--bg-3);
    flex: none;
    object-fit: cover;
  }
  .txt {
    min-width: 0;
    flex: 1;
    display: flex;
    flex-direction: column;
  }
  .name {
    font-size: 13px;
    font-weight: 500;
  }
  .sub {
    font-size: 12px;
    color: var(--fg-2);
  }
  .dur {
    font-size: 12px;
    color: var(--fg-3);
  }
  .rm {
    opacity: 0;
    margin-right: 4px;
  }
  .row:hover .rm {
    opacity: 1;
  }
  .rm :global(svg) {
    width: 14px;
    height: 14px;
  }
  @media (max-width: 640px) {
    .main {
      height: 58px;
    }
    .rm {
      opacity: 1;
      width: 40px;
      height: 40px;
    }
    .name {
      font-size: 15px;
    }
  }
</style>
