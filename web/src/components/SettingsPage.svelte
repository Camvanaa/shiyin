<script lang="ts">
  import SettingsBody from './SettingsBody.svelte';
  import { app } from '../lib/app.svelte';
  import { sourceShort } from '../lib/format';

  // 移动端：iOS「设置」风格的分组列表页面，一屏到底自然滚动

  /** 已有凭证的音源：网易云按账号算，其它按 cookie 算 */
  const loggedIn = $derived(app.sources.filter((s) => (s.id === 'netease' ? !!app.account : s.hasCookie)).map((s) => sourceShort(s.id)));
  const accSub = $derived(
    loggedIn.length ? `已登录 ${loggedIn.join(' · ')}${loggedIn.length < app.sources.length ? '，点击管理其它音源' : ''}` : '登录后可播放会员曲库与高音质',
  );
</script>

<div class="page-settings">
  <header class="page-head">
    <h1>设置</h1>
  </header>

  <button class="account" onclick={() => (app.sheet = 'login')}>
    <span class="avatar">
      {#if app.account}
        <img src={app.account.avatar + '?param=96y96'} alt="" width="52" height="52" />
      {:else}
        <svg viewBox="0 0 24 24" width="26" height="26" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="12" cy="8.5" r="3.5" /><path d="M5 20c1.2-3.5 4-5 7-5s5.8 1.5 7 5" />
        </svg>
      {/if}
    </span>
    <span class="acc-txt">
      <span class="acc-name">{app.account?.nickname ?? '登录音源账号'}</span>
      <span class="acc-sub">{accSub}</span>
    </span>
    <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="chev"><path d="m9 6 6 6-6 6" /></svg>
  </button>

  <SettingsBody pane="all" />
</div>

<style>
  .page-settings {
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
  .account {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 12px 14px 12px 14px;
    margin-bottom: 28px;
    border-radius: var(--r-lg);
    background: var(--bg-2);
    text-align: left;
    color: var(--fg-3);
    transition: background-color var(--t-fast) var(--ease);
  }
  .account:active {
    background: var(--press);
  }
  .avatar {
    width: 52px;
    height: 52px;
    border-radius: 50%;
    overflow: hidden;
    display: grid;
    place-items: center;
    background: var(--bg-3);
    color: var(--fg-2);
    flex: none;
  }
  .avatar img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .acc-txt {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .acc-name {
    font-size: 17px;
    font-weight: 600;
    color: var(--fg);
    letter-spacing: -0.015em;
  }
  .acc-sub {
    font-size: 13px;
    color: var(--fg-2);
  }
  .chev {
    flex: none;
  }
</style>
