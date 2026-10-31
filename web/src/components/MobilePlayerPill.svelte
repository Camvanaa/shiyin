<script lang="ts">
  import Icon from './Icon.svelte';
  import { player } from '../lib/player.svelte';
  import { artists, img } from '../lib/format';

  const t = $derived(player.current);

  // 进度百分比
  const progressRatio = $derived(
    player.duration > 0 ? Math.min(1, Math.max(0, player.time / player.duration)) : 0
  );

  function handleOpenNowPlaying() {
    player.expanded = true;
  }

  function handleTogglePlay(e: MouseEvent) {
    e.stopPropagation();
    player.toggle();
  }

  function handleNext(e: MouseEvent) {
    e.stopPropagation();
    player.next();
  }
</script>

{#if t}
  <div
    class="pill-wrap"
    role="button"
    tabindex="0"
    onclick={handleOpenNowPlaying}
    onkeydown={(e) => e.key === 'Enter' && handleOpenNowPlaying()}
    aria-label="打开播放器：{t.name}"
  >
    <div class="pill">
      <!-- 封面 -->
      <div class="art">
        {#key t.id}
          <img src={img(t.cover, 56)} alt="" width="44" height="44" />
        {/key}
      </div>

      <!-- 曲目信息 -->
      <div class="info">
        <span class="name ellipsis">{t.name}</span>
        <span class="artist ellipsis">{artists(t)}</span>
      </div>

      <!-- 控制按钮 -->
      <div class="actions">
        <button
          class="btn-icon"
          onclick={handleTogglePlay}
          aria-label={player.playing ? '暂停' : '播放'}
        >
          {#if player.loading}
            <span class="spinner-sm"></span>
          {:else}
            <Icon name={player.playing ? 'pause' : 'play'} size={20} />
          {/if}
        </button>

        <button
          class="btn-icon"
          onclick={handleNext}
          aria-label="下一首"
        >
          <Icon name="next" size={20} />
        </button>
      </div>

      <!-- 底部微细进度条 -->
      <div class="progress-bar">
        <div class="fill" style:transform="scaleX({progressRatio})"></div>
      </div>
    </div>
  </div>
{/if}

<style>
  .pill-wrap {
    position: fixed;
    left: 8px;
    right: 8px;
    bottom: calc(50px + var(--sab) + 8px);
    z-index: 65;
    animation: pill-in 320ms var(--ease-out) both;
    outline: none;
    -webkit-tap-highlight-color: transparent;
  }
  @keyframes pill-in {
    from {
      opacity: 0;
      transform: translateY(16px) scale(0.96);
    }
  }

  .pill {
    position: relative;
    display: flex;
    align-items: center;
    gap: 12px;
    height: 56px;
    padding: 0 10px 0 6px;
    border-radius: 14px;
    background: var(--bg-elev);
    box-shadow: 0 4px 24px rgba(0, 0, 0, 0.16), 0 0 0 0.5px var(--line-2);
    overflow: hidden;
    transition: transform var(--t-fast) var(--ease);
  }
  .pill:active {
    transform: scale(0.985);
  }

  .art {
    width: 44px;
    height: 44px;
    border-radius: 10px;
    overflow: hidden;
    background: var(--bg-3);
    flex: none;
    box-shadow: var(--shadow-sm);
  }
  .art img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .name {
    font-size: 14px;
    font-weight: 600;
    color: var(--fg);
    letter-spacing: -0.01em;
  }
  .artist {
    font-size: 12px;
    color: var(--fg-2);
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .btn-icon {
    width: 40px;
    height: 40px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    color: var(--fg);
    transition: background-color var(--t-fast) var(--ease);
  }
  .btn-icon:active {
    background: var(--press);
    transform: scale(0.92);
  }

  .progress-bar {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    height: 2px;
    background: var(--line);
  }
  .fill {
    height: 100%;
    background: var(--accent);
    transform-origin: left;
    transition: transform 200ms linear;
  }

  .spinner-sm {
    width: 16px;
    height: 16px;
    border-radius: 50%;
    border: 2px solid var(--line-2);
    border-top-color: var(--fg);
    animation: spin 0.8s linear infinite;
  }
</style>
