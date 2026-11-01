<script lang="ts">
  import DownloadsBody from './DownloadsBody.svelte';

  // 移动端：全屏下载页面
</script>

<div class="page-downloads">
  <header class="page-head">
    <h1>下载</h1>
  </header>
  <DownloadsBody />
</div>

<style>
  .page-downloads {
    padding: 0 0 24px;
  }
  .page-head {
    padding: 2px 0 14px;
  }
  h1 {
    margin: 0;
    font: 700 28px/1.1 var(--font-display);
    letter-spacing: -0.028em;
  }
</style>
