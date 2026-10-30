<script lang="ts">
  import Icon from './Icon.svelte';
  import Sheet from './Sheet.svelte';
  import { app } from '../lib/app.svelte';
  import { api, isTauri } from '../lib/transport';
  import { toast } from '../lib/toast.svelte';

  let platform = $state<'netease' | 'qq' | 'kuwo'>('netease');
  let extCookie = $state('');
  import type { QrState } from '../lib/types';

  let mode = $state<'qr' | 'cookie'>('qr');
  let svg = $state('');
  let qrState = $state<QrState | 'loading' | 'error'>('loading');
  let cookie = $state('');
  let saving = $state(false);
  let errMsg = $state('');

  const open = $derived(app.sheet === 'login');

  // 打开时生成二维码并轮询状态，关闭或切换模式时停止
  $effect(() => {
    if (!open || mode !== 'qr' || app.account) return;
    let stopped = false;
    let timer: ReturnType<typeof setTimeout>;
    const start = async () => {
      qrState = 'loading';
      try {
        const q = await api.qrStart();
        if (stopped) return;
        svg = q.svg;
        qrState = 'waiting';
        const poll = async () => {
          if (stopped) return;
          try {
            const r = await api.qrCheck(q.key);
            if (stopped) return;
            qrState = r.state;
            if (r.state === 'success') {
              await app.refreshAccount();
              toast.success(`欢迎，${app.account?.nickname ?? ''}`);
              app.sheet = null;
              return;
            }
            if (r.state === 'expired') return;
          } catch (e) {
            errMsg = (e as Error).message;
          }
          timer = setTimeout(poll, 2000);
        };
        timer = setTimeout(poll, 2000);
      } catch (e) {
        if (!stopped) {
          qrState = 'error';
          errMsg = (e as Error).message;
        }
      }
    };
    start();
    return () => {
      stopped = true;
      clearTimeout(timer);
    };
  });

  function refresh() {
    // 切换模式会重新触发上面的 effect
    mode = 'cookie';
    setTimeout(() => (mode = 'qr'));
  }

  async function saveCookie() {
    if (!cookie.trim()) return;
    saving = true;
    try {
      const r = await api.setCookie(cookie.trim());
      app.account = r.account;
      app.hasCredential = true;
      cookie = '';
      toast.success(`欢迎，${r.account.nickname}`);
      app.sheet = null;
    } catch (e) {
      toast.error('登录失败', (e as Error).message);
    } finally {
      saving = false;
    }
  }

  const vip = (t: number) => (t >= 11 ? '黑胶 SVIP' : t > 0 ? '黑胶 VIP' : '普通用户');
</script>

<Sheet {open} onclose={() => (app.sheet = null)} title="音源账号与凭证" width={420}>
  <div class="seg plat" role="tablist">
    {#each [['netease', '网易云'], ['qq', 'QQ音乐'], ['kuwo', '酷我']] as const as [id, label]}
      <button class:on={platform === id} onclick={() => ((platform = id), (extCookie = ''))} role="tab" aria-selected={platform === id}>{label}</button>
    {/each}
  </div>
  {#if platform !== 'netease'}
    {@const info = app.sources.find((x) => x.id === platform)}
    <div class="cookie">
      <p class="muted">
        {platform === 'qq' ? '粘贴 y.qq.com 已登录的 Cookie（需含 uin 与 qqmusic_key）以访问绿钻曲库。' : '粘贴 kuwo.cn 已登录的 Cookie。'}当前状态：{info?.hasCookie ? '已配置' : '未配置'}。
      </p>
      <textarea class="input" rows="4" bind:value={extCookie} placeholder="粘贴 Cookie" spellcheck="false"></textarea>
      <div class="row-btns">
        {#if info?.hasCookie}
          <button class="btn lg" onclick={async () => { await api.setSourceCookie(platform, null); app.sources = await api.sources(); toast.info('已清除凭证'); }}>清除</button>
        {/if}
        <button class="btn primary lg" disabled={!extCookie.trim()} onclick={async () => { await api.setSourceCookie(platform, extCookie.trim()); extCookie = ''; app.sources = await api.sources(); app.loadHome(); toast.success('凭证已保存'); }}>保存</button>
      </div>
    </div>
  {:else}
  {#if app.account}
    <div class="me">
      <img src={app.account.avatar + '?param=128y128'} alt="" width="72" height="72" />
      <div class="nick">{app.account.nickname}</div>
      <div class="muted">{vip(app.account.vipType)}</div>
      <button class="btn lg" onclick={() => app.logout()}>退出登录</button>
    </div>
  {:else}
    <div class="seg" role="tablist">
      <button class:on={mode === 'qr'} onclick={() => (mode = 'qr')} role="tab" aria-selected={mode === 'qr'}>扫码登录</button>
      <button class:on={mode === 'cookie'} onclick={() => (mode = 'cookie')} role="tab" aria-selected={mode === 'cookie'}>MUSIC_U</button>
      <span class="pill" style:transform="translateX({mode === 'qr' ? 0 : 100}%)"></span>
    </div>

    {#if mode === 'qr'}
      <div class="qr-wrap">
        <div class="qr" class:dim={qrState !== 'waiting'}>
          {#if svg}{@html svg}{/if}
          {#if qrState === 'loading'}
            <div class="over"><span class="spinner"></span></div>
          {:else if qrState === 'scanned'}
            <div class="over ok"><Icon name="check" size={28} stroke={2.4} /><span>已扫码，请在手机上确认</span></div>
          {:else if qrState === 'expired' || qrState === 'error'}
            <div class="over">
              <span>{qrState === 'expired' ? '二维码已过期' : '二维码获取失败'}</span>
              <button class="btn" onclick={refresh}>刷新</button>
            </div>
          {/if}
        </div>
        <p class="muted">打开网易云音乐 App，扫描二维码登录</p>
        {#if errMsg && qrState === 'error'}<p class="err">{errMsg}</p>{/if}
      </div>
    {:else}
      <div class="cookie">
        <p class="muted">从已登录的网页版 Cookie 中复制 <code>MUSIC_U</code> 的值。凭证只保存在{isTauri ? '本机' : '你部署的服务端'}，不会发送到其他地方。</p>
        <textarea class="input" rows="4" bind:value={cookie} placeholder="粘贴 MUSIC_U" spellcheck="false"></textarea>
        <button class="btn primary lg" onclick={saveCookie} disabled={saving || !cookie.trim()}>
          {#if saving}<span class="spinner"></span>{/if}验证并保存
        </button>
      </div>
    {/if}
  {/if}
  {/if}
</Sheet>

<style>
  .me {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    padding: 12px 0 8px;
  }
  .me img {
    border-radius: 50%;
    margin-bottom: 10px;
  }
  .nick {
    font: 600 19px var(--font-display);
    letter-spacing: -0.015em;
  }
  .me .btn {
    margin-top: 20px;
  }
  .seg {
    position: relative;
    display: grid;
    grid-template-columns: 1fr 1fr;
    padding: 2px;
    border-radius: 9px;
    background: var(--bg-2);
    margin-bottom: 20px;
  }
  .seg button {
    position: relative;
    z-index: 1;
    height: 28px;
    font-size: 13px;
    font-weight: 500;
    color: var(--fg-2);
    transition: color var(--t) var(--ease);
  }
  .seg button.on {
    color: var(--fg);
  }
  .pill {
    position: absolute;
    left: 2px;
    top: 2px;
    bottom: 2px;
    width: calc(50% - 2px);
    border-radius: 7px;
    background: var(--bg-elev);
    box-shadow: var(--shadow-sm);
    transition: transform var(--t) var(--ease);
  }
  .qr-wrap {
    text-align: center;
  }
  .qr {
    position: relative;
    width: 200px;
    height: 200px;
    margin: 4px auto 0;
    padding: 14px;
    border-radius: var(--r-lg);
    background: #fff;
    color: #000;
    box-shadow: var(--shadow-sm);
  }
  .qr :global(svg) {
    width: 100%;
    height: 100%;
    display: block;
    transition: opacity var(--t) var(--ease), filter var(--t) var(--ease);
  }
  .qr.dim :global(svg) {
    opacity: 0.12;
  }
  .over {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    font-size: 13px;
    font-weight: 500;
    color: #1d1d1f;
    animation: fade var(--t) var(--ease);
  }
  .over .btn {
    background: #f0f0f2;
    color: #1d1d1f;
  }
  .over.ok {
    color: var(--ok);
  }
  .qr-wrap p {
    margin: 16px 0 4px;
    font-size: 13px;
  }
  .err {
    color: var(--accent);
    font-size: 12px;
  }
  .cookie p {
    margin: 0 0 12px;
    font-size: 13px;
  }
  code {
    font-family: var(--mono);
    font-size: 12px;
    padding: 1px 4px;
    border-radius: 4px;
    background: var(--bg-2);
  }
  textarea.input {
    height: auto;
    padding: 10px 12px;
    font-family: var(--mono);
    font-size: 12px;
    resize: none;
    word-break: break-all;
  }
  .seg.plat {
    grid-template-columns: 1fr 1fr 1fr;
    margin-bottom: 14px;
  }
  .seg.plat button.on {
    background: var(--bg-elev);
    border-radius: 7px;
    box-shadow: var(--shadow-sm);
  }
  .row-btns {
    display: flex;
    gap: 8px;
    margin-top: 12px;
  }
  .row-btns .btn {
    flex: 1;
  }
  .cookie .btn {
    width: 100%;
    margin-top: 12px;
  }
</style>
