<script lang="ts">
  import Icon from './Icon.svelte';
  import { player } from '../lib/player.svelte';
  import { artists, img } from '../lib/format';

  const t = $derived(player.current);
  const ratio = $derived(player.duration > 0 ? Math.min(1, Math.max(0, player.time / player.duration)) : 0);

  function open() {
    player.expanded = true;
  }
  function stop(fn: () => void) {
    return (e: MouseEvent) => {
      e.stopPropagation();
      fn();
    };
  }
</script>

{#if t && !player.expanded}
  <div class="pill-wrap">
    <div class="pill" role="button" tabindex="0" onclick={open} onkeydown={(e) => e.key === 'Enter' && open()} aria-label="打开播放器：{t.name}">
      <div class="art">
        {#key t.id}
          <img src={img(t.cover, 56)} alt="" width="44" height="44" decoding="async" />
        {/key}
      </div>
      <div class="info">
        <span class="name ellipsis">{t.name}</span>
        <span class="artist ellipsis">{artists(t)}</span>
      </div>
      <div class="actions">
        <button class="ctl" onclick={stop(() => player.toggle())} aria-label={player.playing ? '暂停' : '播放'}>
          {#if player.loading}
            <span class="spinner"></span>
          {:else}
            <Icon name={player.playing ? 'pause' : 'play'} size={22} />
          {/if}
        </button>
        <button class="ctl" onclick={stop(() => player.next())} aria-label="下一首">
          <Icon name="next" size={22} />
        </button>
      </div>
      <div class="progress"><div class="fill" style:transform="scaleX({ratio})"></div></div>
    </div>
  </div>
{/if}

<style>
  .pill-wrap {
    position: fixed;
    left: 8px;
    right: 8px;
    bottom: calc(var(--tabbar) + var(--sab) + 8px);
    z-index: 65;
    animation: pill-in var(--t-slow) var(--ease-out) both;
  }
  @keyframes pill-in {
    from {
      opacity: 0;
      transform: translateY(12px);
    }
  }
  .pill {
    position: relative;
    display: flex;
    align-items: center;
    gap: 12px;
    height: 56px;
    padding: 0 8px 0 6px;
    border-radius: 14px;
    background: color-mix(in srgb, var(--bg-elev) 94%, transparent);
    -webkit-backdrop-filter: saturate(180%) blur(20px);
    backdrop-filter: saturate(180%) blur(20px);
    box-shadow: var(--shadow);
    overflow: hidden;
    outline: none;
    transition: transform var(--t-fast) var(--ease);
  }
  .pill:active {
    transform: scale(0.985);
  }
  .art {
    width: 44px;
    height: 44px;
    border-radius: 9px;
    overflow: hidden;
    background: var(--bg-3);
    flex: none;
    box-shadow: var(--shadow-sm);
  }
  .art img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    animation: fade var(--t) var(--ease);
  }
  .info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .name {
    font-size: 14px;
    font-weight: 600;
    letter-spacing: -0.01em;
  }
  .artist {
    font-size: 12px;
    color: var(--fg-2);
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 2px;
  }
  .ctl {
    width: 42px;
    height: 42px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    color: var(--fg);
    transition: transform var(--t-fast) var(--ease), background-color var(--t-fast) var(--ease);
  }
  .ctl:active {
    background: var(--press);
    transform: scale(0.9);
  }
  .ctl .spinner {
    border-top-color: var(--fg);
  }
  .progress {
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
    transition: transform 160ms linear;
  }
</style>
