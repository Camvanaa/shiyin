<script lang="ts">
  import Sheet from './Sheet.svelte';
  import DownloadsBody from './DownloadsBody.svelte';
  import { app } from '../lib/app.svelte';
  import { api } from '../lib/transport';

  // 桌面：右侧抽屉。移动端的下载是独立页面（DownloadsPage）。
  const finished = $derived(app.jobs.filter((j) => !['running', 'waiting', 'paused'].includes(j.status)));
</script>

<Sheet open={app.sheet === 'downloads'} onclose={() => (app.sheet = null)} title="下载" side width={420}>
  {#snippet actions()}
    {#if finished.length}
      <button class="btn plain" onclick={() => api.clearJobs()}>清除已完成</button>
    {/if}
  {/snippet}
  <DownloadsBody />
</Sheet>
