<script lang="ts">
  import type { Snippet } from 'svelte';
  import Icon from './Icon.svelte';

  // 浮层：side = 右侧抽屉；center = 居中面板
  let {
    open,
    onclose,
    title,
    side = false,
    width = 520,
    dismissible = true,
    children,
    actions,
  }: {
    open: boolean;
    onclose: () => void;
    title: string;
    side?: boolean;
    width?: number;
    /** 为 false 时不显示关闭按钮，点遮罩和 Esc 也不关闭 */
    dismissible?: boolean;
    children: Snippet;
    actions?: Snippet;
  } = $props();

  // 关闭动画：保留 DOM 直到动画结束
  let mounted = $state(false);
  let closing = $state(false);
  $effect(() => {
    if (open) {
      mounted = true;
      closing = false;
    } else if (mounted) {
      closing = true;
      const t = setTimeout(() => {
        mounted = false;
        closing = false;
      }, 260);
      return () => clearTimeout(t);
    }
  });

  function onKey(e: KeyboardEvent) {
    if (open && dismissible && e.key === 'Escape') {
      e.stopPropagation();
      onclose();
    }
  }
</script>

<svelte:window onkeydown={onKey} />

{#if mounted}
  <div class="scrim" class:closing onclick={() => dismissible && onclose()} role="presentation"></div>
  <div class="sheet" class:side class:closing style:--w="{width}px" role="dialog" aria-modal="true" aria-label={title}>
    <div class="sheet-grabber-wrap"><div class="sheet-grabber"></div></div>
    <header>
      <h2>{title}</h2>
      <div class="acts">
        {#if actions}{@render actions()}{/if}
        {#if dismissible}
          <button class="icon-btn" onclick={onclose} aria-label="关闭"><Icon name="close" /></button>
        {/if}
      </div>
    </header>
    <div class="content">{@render children()}</div>
  </div>
{/if}

<style>
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 70;
    background: var(--scrim);
    animation: fade var(--t) var(--ease) both;
  }
  .scrim.closing {
    animation: fade-out var(--t) var(--ease) both;
  }
  .sheet {
    position: fixed;
    z-index: 71;
    left: 50%;
    top: 50%;
    width: min(var(--w), calc(100vw - 32px));
    max-height: min(720px, calc(100vh - 64px));
    display: flex;
    flex-direction: column;
    border-radius: var(--r-xl);
    background: var(--bg-elev);
    box-shadow: var(--shadow);
    transform: translate(-50%, -50%);
    animation: center-in var(--t-slow) var(--ease) both;
  }
  .sheet.closing {
    animation: center-out var(--t) var(--ease) both;
  }
  .sheet.side {
    left: auto;
    right: 8px;
    top: 8px;
    bottom: 8px;
    max-height: none;
    width: min(var(--w), calc(100vw - 16px));
    transform: none;
    border-radius: var(--r-lg);
    animation: side-in var(--t-slow) var(--ease) both;
  }
  .sheet.side.closing {
    animation: side-out var(--t) var(--ease) both;
  }
  @keyframes center-in {
    from {
      opacity: 0;
      transform: translate(-50%, -48%) scale(0.97);
    }
  }
  @keyframes center-out {
    to {
      opacity: 0;
      transform: translate(-50%, -49%) scale(0.98);
    }
  }
  @keyframes side-in {
    from {
      transform: translateX(calc(100% + 16px));
    }
  }
  @keyframes side-out {
    to {
      transform: translateX(calc(100% + 16px));
    }
  }
  @keyframes fade-out {
    to {
      opacity: 0;
    }
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    height: 56px;
    padding: 0 12px 0 20px;
    flex: none;
  }
  h2 {
    margin: 0;
    font: 600 17px var(--font-display);
    letter-spacing: -0.015em;
  }
  .acts {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .content {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 0 20px 20px;
  }
  .sheet-grabber-wrap {
    display: none;
  }

  @media (max-width: 640px) {
    .sheet-grabber-wrap {
      display: flex;
      justify-content: center;
      padding: 8px 0 0;
    }
    .sheet-grabber {
      width: 36px;
      height: 4.5px;
      border-radius: 99px;
      background: var(--line-2);
    }

    .sheet, .sheet.side {
      left: 0 !important;
      right: 0 !important;
      bottom: 0 !important;
      top: auto !important;
      width: 100% !important;
      max-height: 88vh !important;
      border-radius: 20px 20px 0 0 !important;
      transform: none !important;
      padding-bottom: var(--sab);
      animation: bottom-sheet-in 320ms var(--ease-out) both !important;
    }

    .sheet.closing, .sheet.side.closing {
      animation: bottom-sheet-out 240ms var(--ease) both !important;
    }

    @keyframes bottom-sheet-in {
      from {
        transform: translateY(100%) !important;
      }
      to {
        transform: translateY(0) !important;
      }
    }

    @keyframes bottom-sheet-out {
      to {
        transform: translateY(100%) !important;
      }
    }

    header {
      height: 48px;
      padding: 0 14px 0 18px;
    }
    .content {
      padding: 0 16px 16px;
    }
  }

</style>
