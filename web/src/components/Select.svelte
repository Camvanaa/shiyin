<script lang="ts" generics="T extends string | number">
  import Icon from './Icon.svelte';

  // 自定义下拉：桌面为锚定弹出菜单，移动端为底部动作表。替代原生 <select>，
  // 各平台外观一致，并与应用其它弹层共用同一套动效。
  let {
    value = $bindable(),
    options,
    label = '',
    compact = false,
    align = 'right',
    onchange,
  }: {
    value: T;
    options: { value: T; label: string; hint?: string }[];
    /** 无障碍名称 */
    label?: string;
    /** 紧凑：无背景底，只显示文字 + 箭头（用于搜索框内） */
    compact?: boolean;
    /** 菜单对齐触发按钮的哪一侧 */
    align?: 'left' | 'right';
    onchange?: (v: T) => void;
  } = $props();

  let open = $state(false);
  let closing = $state(false);
  let cursor = $state(-1);
  let btn = $state<HTMLButtonElement>();
  let menu = $state<HTMLElement>();
  /** 弹出方向：空间不够时向上 */
  let up = $state(false);
  /** 菜单锚点（视口坐标），用 fixed 定位脱离祖先的 overflow 裁剪 */
  let anchor = $state({ top: 0, bottom: 0, left: 0, right: 0, width: 0 });

  const current = $derived(options.find((o) => o.value === value));

  function show() {
    cursor = Math.max(0, options.findIndex((o) => o.value === value));
    const r = btn?.getBoundingClientRect();
    if (r) anchor = { top: r.top, bottom: r.bottom, left: r.left, right: innerWidth - r.right, width: r.width };
    up = !!r && innerHeight - r.bottom < Math.min(options.length, 7) * 34 + 24 && r.top > innerHeight / 2;
    open = true;
    closing = false;
    requestAnimationFrame(() => menu?.querySelector<HTMLElement>('[data-i="' + cursor + '"]')?.focus());
  }
  function hide() {
    if (!open) return;
    closing = true;
    setTimeout(() => {
      open = false;
      closing = false;
    }, 160);
    btn?.focus();
  }
  function pick(v: T) {
    if (v !== value) {
      value = v;
      onchange?.(v);
    }
    hide();
  }
  function onKey(e: KeyboardEvent) {
    if (!open) {
      if (e.key === 'ArrowDown' || e.key === 'ArrowUp' || e.key === 'Enter' || e.key === ' ') {
        e.preventDefault();
        show();
      }
      return;
    }
    if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      hide();
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      cursor = (cursor + 1) % options.length;
      menu?.querySelector<HTMLElement>('[data-i="' + cursor + '"]')?.focus();
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      cursor = (cursor - 1 + options.length) % options.length;
      menu?.querySelector<HTMLElement>('[data-i="' + cursor + '"]')?.focus();
    } else if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      pick(options[cursor].value);
    } else if (e.key === 'Tab') {
      hide();
    }
  }
  /** 挂到 body：祖先若有 transform（如居中面板），fixed 会相对该祖先定位而错位 */
  function portal(node: HTMLElement) {
    document.body.appendChild(node);
    return {
      destroy() {
        node.remove();
      },
    };
  }
  function onAnyScroll(e: Event) {
    // 菜单自身滚动不关闭；祖先 / 页面滚动时关闭（fixed 菜单不会跟随锚点）
    if (open && !menu?.contains(e.target as Node)) hide();
  }
  function onDocClick(e: MouseEvent) {
    if (!open) return;
    const t = e.target as Node;
    if (btn?.contains(t) || menu?.contains(t)) return;
    hide();
  }
</script>

<svelte:window onclick={onDocClick} onresize={hide} onscrollcapture={onAnyScroll} />

<div class="sel" class:compact class:open onkeydown={onKey} role="presentation">
  <button
    bind:this={btn}
    class="trigger"
    type="button"
    onclick={() => (open ? hide() : show())}
    aria-haspopup="listbox"
    aria-expanded={open}
    aria-label={label || undefined}
  >
    <span class="val ellipsis">{current?.label ?? ''}</span>
    <Icon name="chevronDown" size={13} stroke={2.4} />
  </button>

  {#if open}
    <div use:portal class="scrim" class:closing onclick={hide} role="presentation"></div>
    <div
      use:portal
      bind:this={menu}
      class="menu"
      class:closing
      class:up
      class:left={align === 'left'}
      style:--a-top="{anchor.top}px"
      style:--a-bottom="{anchor.bottom}px"
      style:--a-left="{anchor.left}px"
      style:--a-right="{anchor.right}px"
      style:--a-w="{anchor.width}px"
      role="listbox"
      aria-label={label || undefined}
      tabindex="-1"
    >
      {#if label}<div class="sheet-title">{label}</div>{/if}
      {#each options as o, i (o.value)}
        <button
          class="opt"
          class:on={o.value === value}
          class:cur={i === cursor}
          data-i={i}
          role="option"
          aria-selected={o.value === value}
          tabindex="-1"
          onclick={() => pick(o.value)}
          onmouseenter={() => (cursor = i)}
        >
          <span class="check">{#if o.value === value}<Icon name="check" size={14} stroke={2.6} />{/if}</span>
          <span class="txt">
            <span class="ellipsis">{o.label}</span>
            {#if o.hint}<small class="ellipsis">{o.hint}</small>{/if}
          </span>
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .sel {
    position: relative;
    display: inline-flex;
    flex: none;
  }
  .trigger {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 28px;
    padding: 0 8px 0 10px;
    border-radius: var(--r-sm);
    background: var(--bg-elev);
    box-shadow: var(--shadow-sm);
    font-size: var(--fs-footnote);
    color: var(--fg);
    max-width: 220px;
    transition: background-color var(--t-fast) var(--ease), box-shadow var(--t-fast) var(--ease), transform var(--t-fast) var(--ease);
  }
  .trigger:hover {
    background: var(--bg-3);
  }
  .trigger:active {
    transform: scale(0.98);
  }
  .open .trigger {
    box-shadow: 0 0 0 1px var(--line-2), 0 0 0 4px var(--accent-soft);
  }
  .trigger :global(svg) {
    color: var(--fg-3);
    flex: none;
    transition: transform var(--t) var(--ease);
  }
  .open .trigger :global(svg) {
    transform: rotate(180deg);
  }
  .val {
    min-width: 0;
  }
  .compact .trigger {
    height: 28px;
    padding: 0 4px 0 8px;
    background: transparent;
    box-shadow: none;
    color: var(--fg-2);
    font-weight: 500;
  }
  .compact .trigger:hover,
  .compact.open .trigger {
    color: var(--fg);
    box-shadow: none;
  }

  .scrim {
    display: none;
  }
  .menu {
    position: fixed;
    z-index: var(--z-menu);
    top: calc(var(--a-bottom) + 6px);
    right: var(--a-right);
    min-width: max(var(--a-w), 168px);
    max-height: min(340px, 60vh);
    overflow: auto;
    padding: 5px;
    border-radius: var(--r);
    background: var(--bg-elev);
    box-shadow: var(--shadow);
    outline: none;
    transform-origin: top right;
    animation: menu-in var(--t) var(--ease) both;
    overscroll-behavior: contain;
  }
  .menu.up {
    top: auto;
    bottom: calc(100vh - var(--a-top) + 6px);
    transform-origin: bottom right;
  }
  .menu.left {
    right: auto;
    left: var(--a-left);
    transform-origin: top left;
  }
  .menu.left.up {
    transform-origin: bottom left;
  }
  .menu.closing {
    animation: menu-out 160ms var(--ease) both;
    pointer-events: none;
  }
  @keyframes menu-in {
    from {
      opacity: 0;
      transform: scale(0.96) translateY(-4px);
    }
  }
  @keyframes menu-out {
    to {
      opacity: 0;
      transform: scale(0.97);
    }
  }
  .sheet-title {
    display: none;
  }
  .opt {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 30px;
    padding: 4px 10px 4px 6px;
    border-radius: var(--r-sm);
    font-size: var(--fs-footnote);
    text-align: left;
    color: var(--fg);
    outline: none;
  }
  .opt.cur {
    background: var(--accent);
    color: var(--white);
  }
  .opt.cur small {
    color: rgb(255 255 255 / 0.75);
  }
  .check {
    width: 16px;
    display: grid;
    place-items: center;
    flex: none;
  }
  .txt {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  small {
    font-size: var(--fs-caption2);
    color: var(--fg-3);
  }

  /* ---- 移动端：底部动作表 ---- */
  @media (max-width: 640px) {
    .trigger {
      height: 44px;
      font-size: var(--fs-callout);
      background: transparent;
      box-shadow: none;
      color: var(--fg-2);
      padding: 0 0 0 8px;
    }
    .trigger:hover,
    .open .trigger {
      background: transparent;
      box-shadow: none;
    }
    .open .trigger :global(svg) {
      transform: none;
    }
    .scrim {
      display: block;
      position: fixed;
      inset: 0;
      z-index: var(--z-sheet);
      background: var(--scrim);
      animation: fade var(--t) var(--ease) both;
    }
    .scrim.closing {
      animation: fade-out 160ms var(--ease) both;
    }
    .menu,
    .menu.up {
      position: fixed;
      top: auto;
      left: 8px;
      right: 8px;
      bottom: calc(8px + var(--sab));
      min-width: 0;
      max-height: 70dvh;
      padding: 6px;
      border-radius: var(--r-xl);
      transform-origin: center bottom;
      animation: sheet-up var(--t-slow) var(--ease) both;
    }
    .menu.closing {
      animation: sheet-down 180ms var(--ease) both;
    }
    @keyframes sheet-up {
      from {
        opacity: 0;
        transform: translateY(24px);
      }
    }
    @keyframes sheet-down {
      to {
        opacity: 0;
        transform: translateY(16px);
      }
    }
    .sheet-title {
      display: block;
      padding: 10px 14px 8px;
      font-size: var(--fs-footnote);
      font-weight: 600;
      color: var(--fg-2);
      text-align: center;
    }
    .opt {
      min-height: 48px;
      padding: 6px 14px 6px 10px;
      font-size: var(--fs-body);
      border-radius: var(--r);
    }
    .opt.cur {
      background: transparent;
      color: var(--fg);
    }
    .opt.cur small {
      color: var(--fg-3);
    }
    .opt:active {
      background: var(--press);
    }
    .opt.on {
      color: var(--accent);
    }
    .check {
      width: 20px;
    }
    small {
      font-size: var(--fs-caption);
    }
  }
</style>
