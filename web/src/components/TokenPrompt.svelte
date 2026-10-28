<script lang="ts">
  import Sheet from './Sheet.svelte';
  import { app } from '../lib/app.svelte';
  import { token } from '../lib/transport';

  let value = $state('');

  function save() {
    if (!value.trim()) return;
    token.set(value.trim());
    location.reload();
  }
</script>

<Sheet open={app.sheet === 'token'} onclose={() => {}} dismissible={false} title="需要访问口令" width={380}>
  <p class="muted">这个拾音服务设置了访问口令。口令保存在当前浏览器中。</p>
  <form onsubmit={(e) => (e.preventDefault(), save())}>
    <input type="text" name="username" value="shiyin" autocomplete="username" hidden />
    <input class="input" type="password" bind:value placeholder="访问口令" autocomplete="current-password" />
    <button class="btn primary lg" disabled={!value.trim()}>继续</button>
  </form>
</Sheet>

<style>
  p {
    margin: 0 0 14px;
    font-size: 13px;
  }
  form {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .input {
    height: 38px;
  }
</style>
