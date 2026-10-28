<script lang="ts">
  // 进度 / 音量滑条。拖动期间只更新本地值，松开时提交，避免频繁 seek。
  let {
    value,
    max = 1,
    buffered = 0,
    onchange,
    oninput,
    label,
    thin = false,
  }: {
    value: number;
    max?: number;
    buffered?: number;
    onchange: (v: number) => void;
    oninput?: (v: number | null) => void;
    label: string;
    thin?: boolean;
  } = $props();

  let el = $state<HTMLElement>();
  let drag = $state<number | null>(null);
  const shown = $derived(drag ?? value);
  const ratio = $derived(max > 0 ? Math.min(1, Math.max(0, shown / max)) : 0);
  const bufRatio = $derived(max > 0 ? Math.min(1, buffered / max) : 0);

  function at(e: PointerEvent) {
    const r = el!.getBoundingClientRect();
    return Math.min(1, Math.max(0, (e.clientX - r.left) / r.width)) * max;
  }

  function down(e: PointerEvent) {
    if (e.button !== 0) return;
    el!.setPointerCapture(e.pointerId);
    drag = at(e);
    oninput?.(drag);
  }
  function move(e: PointerEvent) {
    if (drag === null) return;
    drag = at(e);
    oninput?.(drag);
  }
  function up() {
    if (drag === null) return;
    onchange(drag);
    drag = null;
    oninput?.(null);
  }
  function key(e: KeyboardEvent) {
    const step = max / 20;
    if (e.key === 'ArrowRight' || e.key === 'ArrowUp') onchange(Math.min(max, value + step));
    else if (e.key === 'ArrowLeft' || e.key === 'ArrowDown') onchange(Math.max(0, value - step));
    else return;
    e.preventDefault();
    e.stopPropagation();
  }
</script>

<div
  class="slider"
  class:thin
  class:dragging={drag !== null}
  bind:this={el}
  onpointerdown={down}
  onpointermove={move}
  onpointerup={up}
  onpointercancel={up}
  onkeydown={key}
  role="slider"
  tabindex="0"
  aria-label={label}
  aria-valuemin={0}
  aria-valuemax={max}
  aria-valuenow={shown}
>
  <div class="track">
    <div class="buf" style:transform="scaleX({bufRatio})"></div>
    <div class="fill" style:transform="scaleX({ratio})"></div>
  </div>
  <div class="knob" style:left="{ratio * 100}%"></div>
</div>

<style>
  .slider {
    position: relative;
    height: 16px;
    display: flex;
    align-items: center;
    touch-action: none;
    cursor: default;
    --h: 4px;
  }
  .track {
    position: relative;
    width: 100%;
    height: var(--h);
    border-radius: 99px;
    background: var(--line-2);
    overflow: hidden;
    transition: height var(--t-fast) var(--ease);
  }
  .slider:hover .track,
  .dragging .track {
    --h: 6px;
    height: var(--h);
  }
  .buf,
  .fill {
    position: absolute;
    inset: 0;
    transform-origin: left;
    will-change: transform;
  }
  .buf {
    background: var(--line);
  }
  .fill {
    background: var(--fg-2);
    transition: background-color var(--t-fast) var(--ease);
  }
  .slider:hover .fill,
  .dragging .fill {
    background: var(--fg);
  }
  .knob {
    position: absolute;
    top: 50%;
    width: 12px;
    height: 12px;
    margin: -6px 0 0 -6px;
    border-radius: 50%;
    background: #fff;
    box-shadow: 0 0 0 0.5px rgba(0, 0, 0, 0.12), 0 1px 4px rgba(0, 0, 0, 0.25);
    transform: scale(0);
    transition: transform var(--t-fast) var(--ease);
  }
  .slider:hover .knob,
  .dragging .knob,
  .slider:focus-visible .knob {
    transform: scale(1);
  }
  .dragging .knob {
    transform: scale(1.15);
  }
</style>
