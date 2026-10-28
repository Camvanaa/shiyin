<script lang="ts">
  import Icon from './Icon.svelte';
  import Sheet from './Sheet.svelte';
  import { app } from '../lib/app.svelte';
  import { api, isTauri } from '../lib/transport';
  import { toast } from '../lib/toast.svelte';
  import { bytes, img } from '../lib/format';
  import type { JobDetail, JobSummary } from '../lib/types';

  const active = $derived(app.jobs.filter((j) => ['running', 'waiting', 'paused'].includes(j.status)));
  const finished = $derived(app.jobs.filter((j) => !['running', 'waiting', 'paused'].includes(j.status)));

  let expanded = $state<number | null>(null);
  let detail = $state<JobDetail | null>(null);

  async function toggleDetail(j: JobSummary) {
    if (expanded === j.id) {
      expanded = null;
      return;
    }
    expanded = j.id;
    detail = null;
    try {
      detail = await api.job(j.id);
    } catch (e) {
      toast.error('无法读取任务', (e as Error).message);
    }
  }

  // 展开时随推送刷新明细
  $effect(() => {
    const id = expanded;
    if (id === null) return;
    const j = app.jobs.find((x) => x.id === id);
    if (!j) return;
    void j.completed;
    void j.failed;
    void j.status;
    api.job(id).then((d) => {
      if (expanded === id) detail = d;
    }).catch(() => {});
  });

  const statusText: Record<string, string> = {
    waiting: '等待中',
    running: '下载中',
    paused: '已暂停',
    done: '已完成',
    failed: '失败',
    cancelled: '已取消',
  };

  function act(j: JobSummary, a: 'pause' | 'resume' | 'cancel' | 'retry') {
    api.jobAction(j.id, a).catch((e) => toast.error('操作失败', e.message));
  }
</script>

<Sheet open={app.sheet === 'downloads'} onclose={() => (app.sheet = null)} title="下载" side width={420}>
  {#snippet actions()}
    {#if finished.length}
      <button class="btn plain" onclick={() => api.clearJobs()}>清除已完成</button>
    {/if}
  {/snippet}

  {#if !app.jobs.length}
    <div class="empty">
      <Icon name="download" size={36} stroke={1.4} />
      <p>没有下载任务</p>
      <span class="muted">{isTauri ? '文件会保存到设置中的下载目录' : '下载完成后浏览器会自动保存文件'}</span>
    </div>
  {/if}

  {#each [...active, ...finished] as j (j.id)}
    <div class="job" class:open={expanded === j.id}>
      <div class="row">
        <div class="cover">
          {#if j.cover}<img src={img(j.cover, 44)} alt="" width="44" height="44" />{/if}
        </div>
        <button class="main" onclick={() => toggleDetail(j)}>
          <div class="title ellipsis">{j.title}</div>
          <div class="sub tabular">
            <span class="st st-{j.status}">{statusText[j.status]}</span>
            {#if j.total > 1}<span>{j.completed}/{j.total}</span>{/if}
            {#if j.failed}<span class="err">{j.failed} 失败</span>{/if}
            {#if j.status === 'running' && j.speed}<span>{bytes(j.speed)}/s</span>{/if}
          </div>
        </button>
        <div class="ops">
          {#if j.status === 'running' || j.status === 'waiting'}
            <button class="icon-btn" onclick={() => act(j, 'pause')} title="暂停" aria-label="暂停"><Icon name="pause" /></button>
          {:else if j.status === 'paused'}
            <button class="icon-btn" onclick={() => act(j, 'resume')} title="继续" aria-label="继续"><Icon name="play" /></button>
          {/if}
          {#if ['running', 'waiting', 'paused'].includes(j.status)}
            <button class="icon-btn" onclick={() => act(j, 'cancel')} title="取消" aria-label="取消"><Icon name="close" /></button>
          {:else}
            {#if j.failed || j.status === 'cancelled'}
              <button class="icon-btn" onclick={() => act(j, 'retry')} title="重试" aria-label="重试"><Icon name="retry" /></button>
            {/if}
            {#if j.status === 'done' && (isTauri || j.hasArtifact)}
              <button class="icon-btn" onclick={() => api.reveal(j)} title={isTauri ? '在文件夹中显示' : '保存'} aria-label="打开">
                <Icon name={isTauri ? 'folder' : 'download'} />
              </button>
            {/if}
            <button class="icon-btn" onclick={() => api.removeJob(j.id)} title="移除" aria-label="移除"><Icon name="trash" /></button>
          {/if}
        </div>
      </div>
      {#if ['running', 'waiting', 'paused'].includes(j.status)}
        <div class="bar"><div class="fill" class:paused={j.status === 'paused'} style:transform="scaleX({j.progress})"></div></div>
      {:else if j.status === 'done' && !isTauri && !j.hasArtifact && j.total > 1}
        <div class="hint muted">正在打包…</div>
      {/if}

      {#if expanded === j.id}
        <div class="items">
          {#if !detail}
            <div class="spinner"></div>
          {:else}
            {#each detail.items as it (it.id)}
              <div class="item">
                <span class="dot st-{it.status}"></span>
                <span class="ellipsis">{it.name} <span class="muted">· {it.artist}</span></span>
                {#if it.status === 'running' && it.total}
                  <span class="pct tabular">{Math.round((it.done / it.total) * 100)}%</span>
                {/if}
                {#if it.error}<span class="ie" title={it.error}>{it.error}</span>{/if}
              </div>
            {/each}
          {/if}
        </div>
      {/if}
    </div>
  {/each}
</Sheet>

<style>
  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 72px 0;
    color: var(--fg-3);
    text-align: center;
  }
  .empty p {
    margin: 14px 0 4px;
    font-size: 15px;
    font-weight: 500;
    color: var(--fg);
  }
  .empty span {
    font-size: 13px;
  }
  .job {
    padding: 10px 0;
    border-bottom: 0.5px solid var(--line);
    animation: rise var(--t) var(--ease-out);
  }
  .row {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .cover {
    width: 44px;
    height: 44px;
    border-radius: var(--r-sm);
    overflow: hidden;
    background: var(--bg-3);
    flex: none;
  }
  .cover img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .main {
    flex: 1;
    min-width: 0;
    text-align: left;
  }
  .title {
    font-size: 14px;
    font-weight: 500;
  }
  .sub {
    display: flex;
    gap: 8px;
    font-size: 12px;
    color: var(--fg-2);
    margin-top: 1px;
  }
  .st-done {
    color: var(--ok);
  }
  .st-failed,
  .err {
    color: var(--accent);
  }
  .st-running {
    color: var(--fg);
  }
  .ops {
    display: flex;
    gap: 2px;
  }
  .ops :global(svg) {
    width: 16px;
    height: 16px;
  }
  .bar {
    margin: 10px 0 2px 56px;
    height: 3px;
    border-radius: 3px;
    background: var(--line);
    overflow: hidden;
  }
  .fill {
    height: 100%;
    background: var(--accent);
    transform-origin: left;
    transition: transform 300ms var(--ease);
  }
  .fill.paused {
    background: var(--fg-3);
  }
  .hint {
    margin: 6px 0 0 56px;
    font-size: 12px;
  }
  .items {
    margin: 10px 0 0 56px;
    max-height: 260px;
    overflow: auto;
    animation: fade var(--t) var(--ease);
  }
  .item {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 28px;
    font-size: 12px;
  }
  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--fg-3);
    flex: none;
  }
  .dot.st-done {
    background: var(--ok);
  }
  .dot.st-failed {
    background: var(--accent);
  }
  .dot.st-running {
    background: var(--fg);
    animation: pulse 1s var(--ease-in-out) infinite alternate;
  }
  @keyframes pulse {
    to {
      opacity: 0.3;
    }
  }
  .pct {
    margin-left: auto;
    color: var(--fg-2);
  }
  .ie {
    margin-left: auto;
    color: var(--accent);
    max-width: 45%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: none;
  }
</style>
