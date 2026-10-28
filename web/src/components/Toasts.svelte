<script lang="ts">
  import { flip } from 'svelte/animate';
  import { fly } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import Icon from './Icon.svelte';
  import { toast } from '../lib/toast.svelte';
</script>

<div class="stack" aria-live="polite">
  {#each toast.items as t (t.id)}
    <div
      class="toast {t.kind}"
      animate:flip={{ duration: 280, easing: cubicOut }}
      in:fly={{ y: 16, duration: 320, easing: cubicOut }}
      out:fly={{ y: 8, duration: 200, opacity: 0 }}
    >
      <span class="ico">
        {#if t.kind === 'success'}<Icon name="check" stroke={2.4} />{:else if t.kind === 'error'}<Icon name="close" stroke={2.4} />{:else}<Icon name="download" stroke={2.2} />{/if}
      </span>
      <div class="txt">
        <div class="title">{t.title}</div>
        {#if t.detail}<div class="detail">{t.detail}</div>{/if}
      </div>
      {#if t.action}
        <button class="btn plain" onclick={() => (t.action!.run(), toast.dismiss(t.id))}>{t.action.label}</button>
      {/if}
    </div>
  {/each}
</div>

<style>
  .stack {
    position: fixed;
    left: 50%;
    bottom: calc(var(--playbar) + 16px);
    transform: translateX(-50%);
    z-index: 90;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    pointer-events: none;
  }
  .toast {
    pointer-events: auto;
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 240px;
    max-width: min(440px, calc(100vw - 32px));
    padding: 10px 10px 10px 12px;
    border-radius: var(--r-lg);
    background: var(--bg-elev);
    box-shadow: var(--shadow);
  }
  .ico {
    width: 22px;
    height: 22px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    flex: none;
    background: var(--bg-3);
    color: var(--fg-2);
  }
  .ico :global(svg) {
    width: 12px;
    height: 12px;
  }
  .success .ico {
    background: var(--ok);
    color: #fff;
  }
  .error .ico {
    background: var(--accent);
    color: #fff;
  }
  .txt {
    flex: 1;
    min-width: 0;
  }
  .title {
    font-size: 13px;
    font-weight: 500;
  }
  .detail {
    font-size: 12px;
    color: var(--fg-2);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
