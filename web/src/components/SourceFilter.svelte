<script lang="ts">
  import Select from './Select.svelte';
  import { app } from '../lib/app.svelte';
  import { sourceShort } from '../lib/format';

  // 音源筛选：桌面用下拉（数量增长不换行），移动端用横向滚动胶囊。
  // 不用分段控件——分段控件适合 2–4 个互斥视图，不适合数量会增长的筛选项。
  let { variant = 'auto' }: { variant?: 'auto' | 'chips' | 'select' } = $props();

  const options = $derived([{ value: 'all', label: '全部音源' }, ...app.sources.map((s) => ({ value: s.id, label: s.label, hint: sourceShort(s.id) }))]);
</script>

<div class="sf" class:chips-only={variant === 'chips'} class:select-only={variant === 'select'}>
  <div class="sel-wrap">
    <Select compact align="left" value={app.source} {options} label="音源" onchange={(v) => app.setSource(v)} />
  </div>

  <div class="chips" role="tablist" aria-label="音源">
    {#each app.sourceTabs as [id, label] (id)}
      <button class="chip" class:on={app.source === id} onclick={() => app.setSource(id)} role="tab" aria-selected={app.source === id}>
        {label}
      </button>
    {/each}
  </div>
</div>

<style>
  .sf {
    min-width: 0;
  }
  .sel-wrap {
    display: flex;
    min-width: 0;
  }
  .chips {
    display: none;
  }

  @media (max-width: 640px) {
    .sf:not(.select-only) .sel-wrap {
      display: none;
    }
    .sf:not(.select-only) .chips {
      display: flex;
      gap: 8px;
      overflow-x: auto;
      scrollbar-width: none;
      -webkit-overflow-scrolling: touch;
      scroll-snap-type: x proximity;
      /* 出血到屏幕边缘，滚动时首尾不贴边 */
      margin: 0 -20px;
      padding: 2px 20px;
    }
    .sf:not(.select-only) .chips::-webkit-scrollbar {
      display: none;
    }
  }
  .sf.chips-only .sel-wrap {
    display: none;
  }
  .sf.chips-only .chips {
    display: flex;
    gap: 8px;
    overflow-x: auto;
    scrollbar-width: none;
    -webkit-overflow-scrolling: touch;
    scroll-snap-type: x proximity;
    margin: 0 -20px;
    padding: 2px 20px;
  }
  .sf.chips-only .chips::-webkit-scrollbar {
    display: none;
  }

  .chip {
    flex: none;
    scroll-snap-align: start;
    height: 32px;
    padding: 0 14px;
    border-radius: var(--r-pill);
    background: var(--bg-2);
    color: var(--fg-2);
    font-size: var(--fs-footnote);
    font-weight: 500;
    white-space: nowrap;
    transition:
      background-color var(--t-fast) var(--ease),
      color var(--t-fast) var(--ease),
      transform var(--t-fast) var(--ease);
  }
  .chip:active {
    transform: scale(0.94);
  }
  .chip.on {
    background: var(--accent);
    color: var(--accent-fg);
  }
</style>
