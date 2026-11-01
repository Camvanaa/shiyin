<script lang="ts">
  import Icon from './Icon.svelte';
  import Select from './Select.svelte';
  import Sheet from './Sheet.svelte';
  import { app } from '../lib/app.svelte';
  import { api, isTauri } from '../lib/transport';
  import { toast } from '../lib/toast.svelte';
  import { sourceName, sourceShort } from '../lib/format';

  let platform = $state('netease');
  let extCookie = $state('');
  /** 支持凭证的音源（后端给了 cookieHint 的），网易云始终第一 */
  const platforms = $derived(app.sources.filter((x) => x.id === 'netease' || x.cookieHint));
  import type { QrState } from '../lib/types';

  let mode = $state<'qr' | 'cookie'>('qr');
  let svg = $state('');
  /** 平台直接下发的二维码图片（QQ 为 PNG data URL） */
  let qrImage = $state('');
  /** 当前平台的元信息 */
  const info = $derived(app.sources.find((x) => x.id === platform));
  /** 当前平台是否走扫码：网易云未登录 + 第三方支持扫码 */
  const qrActive = $derived(mode === 'qr' && (platform === 'netease' ? !app.account : !!info?.supportsQr));
  let qrState = $state<QrState | 'loading' | 'error'>('loading');
  let cookie = $state('');
  let saving = $state(false);
  let errMsg = $state('');

  const open = $derived(app.sheet === 'login');

  // 打开时生成二维码并轮询状态，关闭 / 切换平台或模式时停止
  $effect(() => {
    if (!open || !qrActive) return;
    const src = platform;
    const isNetease = src === 'netease';
    let stopped = false;
    let timer: ReturnType<typeof setTimeout>;
    const start = async () => {
      qrState = 'loading';
      svg = '';
      qrImage = '';
      try {
        const q = isNetease ? await api.qrStart() : await api.sourceQrStart(src);
        if (stopped) return;
        svg = q.svg;
        qrImage = q.image ?? '';
        qrState = 'waiting';
        const poll = async () => {
          if (stopped) return;
          try {
            if (isNetease) {
              const r = await api.qrCheck(q.key);
              if (stopped) return;
              qrState = r.state;
              if (r.state === 'success') {
                await app.refreshAccount();
                toast.success(`欢迎，${app.account?.nickname ?? ''}`);
                app.sheet = null;
                return;
              }
            } else {
              const r = await api.sourceQrCheck(src, q.key);
              if (stopped) return;
              qrState = r.state;
              if (r.state === 'success') {
                app.sources = await api.sources();
                app.loadHome();
                toast.success(`${sourceName(src)} 登录成功`);
                app.sheet = null;
                return;
              }
            }
            if (qrState === 'expired') return;
          } catch (e) {
            // 扫码后的换 key 链路失败：展示错误并停止轮询，让用户刷新重试
            qrState = 'error';
            errMsg = (e as Error).message;
            return;
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

  let webLoginBusy = $state(false);
  async function webLogin() {
    if (!api.sourceWebLogin) return;
    webLoginBusy = true;
    try {
      await api.sourceWebLogin(platform);
      app.sources = await api.sources();
      app.loadHome();
      toast.success(`${sourceName(platform)} 登录成功`);
    } catch (e) {
      const msg = (e as Error).message ?? String(e);
      if (!msg.includes('取消')) toast.error('登录失败', msg);
    } finally {
      webLoginBusy = false;
    }
  }
</script>

{#snippet qrView(hint: string)}
  <div class="qr-wrap">
    <div class="qr" class:dim={qrState !== 'waiting'}>
      {#if qrImage}<img src={qrImage} alt="" />{:else if svg}{@html svg}{/if}
      {#if qrState === 'loading'}
        <div class="over"><span class="spinner"></span></div>
      {:else if qrState === 'scanned'}
        <div class="over ok"><Icon name="check" size={28} stroke={2.4} /><span>已扫码，请在手机上确认</span></div>
      {:else if qrState === 'expired' || qrState === 'error'}
        <div class="over">
          <span>{qrState === 'expired' ? '二维码已过期' : '登录失败'}</span>
          <button class="btn" onclick={refresh}>刷新</button>
        </div>
      {/if}
    </div>
    <p class="muted">{hint}</p>
    {#if errMsg && qrState === 'error'}<p class="err">{errMsg}</p>{/if}
  </div>
{/snippet}

<Sheet {open} onclose={() => (app.sheet = null)} title="音源账号与凭证" width={440} size="lg" height={560} bare>
  <div class="wrap">
  <div class="seg-ctl plat" role="tablist">
    {#each platforms as p (p.id)}
      <button class:on={platform === p.id} onclick={() => ((platform = p.id), (extCookie = ''))} role="tab" aria-selected={platform === p.id}>{sourceShort(p.id)}</button>
    {/each}
  </div>
  {#key platform + (platform === 'netease' ? (app.account ? ':me' : ':' + mode) : ':' + mode)}
  <div class="pane">
  {#if platform !== 'netease'}
    {#if info?.supportsQr && !info.hasCookie}
      <div class="seg" role="tablist">
        <button class:on={mode === 'qr'} onclick={() => (mode = 'qr')} role="tab" aria-selected={mode === 'qr'}>扫码登录</button>
        <button class:on={mode === 'cookie'} onclick={() => (mode = 'cookie')} role="tab" aria-selected={mode === 'cookie'}>Cookie</button>
        <span class="pill" style:transform="translateX({mode === 'qr' ? 0 : 100}%)"></span>
      </div>
    {/if}
    {#if info?.supportsQr && !info.hasCookie && mode === 'qr'}
      {@render qrView(`打开手机 QQ，扫描二维码登录 ${info.label}`)}
    {:else}
    <div class="cookie">
      {#if info?.options.length}
        <div class="opts">
          {#each info.options as o (o.key)}
            <div class="opt-row">
              <span>{o.label}</span>
              <Select
                value={o.value}
                options={o.choices.map(([value, label]) => ({ value, label }))}
                label={o.label}
                onchange={async (v) => { await api.setSourceOption(platform, o.key, v); app.sources = await api.sources(); app.loadHome(); toast.success(`${info.label} 已切换到${o.choices.find((c) => c[0] === v)?.[1] ?? v}`); }}
              />
            </div>
          {/each}
        </div>
      {/if}
      {#if info?.supportsWebLogin && !info.hasCookie}
        <button class="btn primary lg weblogin" disabled={webLoginBusy} onclick={webLogin}>
          {#if webLoginBusy}<span class="spinner"></span>等待登录完成…{:else}在应用内登录 {info.label}{/if}
        </button>
        <p class="muted">会打开一个登录窗口，在 {info.label} 官方页面里登录即可（扫码、密码、验证码都在那边完成），登录后窗口自动关闭并保存凭证。密码不经过本应用。</p>
        <div class="or"><span>或手动粘贴</span></div>
      {/if}
      <p class="muted">
        {info?.cookieHint}当前状态：{info?.hasCookie ? '已配置' : '未配置'}。
      </p>
      <textarea class="input" rows="4" bind:value={extCookie} placeholder="粘贴 Cookie" spellcheck="false"></textarea>
      <div class="row-btns">
        {#if info?.hasCookie}
          <button class="btn lg" onclick={async () => { await api.setSourceCookie(platform, null); app.sources = await api.sources(); toast.info('已清除凭证'); }}>清除</button>
        {/if}
        <button class="btn primary lg" disabled={!extCookie.trim()} onclick={async () => { await api.setSourceCookie(platform, extCookie.trim()); extCookie = ''; app.sources = await api.sources(); app.loadHome(); toast.success('凭证已保存'); }}>保存</button>
      </div>
    </div>
    {/if}
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
      {@render qrView('打开网易云音乐 App，扫描二维码登录')}
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
  </div>
  {/key}
  </div>
</Sheet>

<style>
  /* 恒定尺寸画布：切换平台 / 登录方式时只有内容淡入，壳体不动 */
  .wrap {
    height: 100%;
    display: flex;
    flex-direction: column;
    padding: 0 20px 20px;
    overflow: auto;
  }
  .pane {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    animation: fade var(--t) var(--ease) both;
  }
  /* 已登录态内容少，居中更稳 */
  .pane:has(.me) {
    justify-content: center;
  }
  .plat {
    flex: none;
  }
  .weblogin {
    width: 100%;
    margin-bottom: 8px;
  }
  .or {
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 14px 0 10px;
    font-size: 12px;
    color: var(--fg-3);
  }
  .or::before,
  .or::after {
    content: '';
    flex: 1;
    height: 0.5px;
    background: var(--line);
  }
  .opts {
    border-radius: var(--r);
    background: var(--bg-2);
    margin-bottom: 12px;
  }
  .opt-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    min-height: 44px;
    padding: 6px 12px 6px 14px;
    font-size: 13px;
  }
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
  .qr img {
    width: 100%;
    height: 100%;
    display: block;
    /* 平台下发的小尺寸 PNG 放大时保持像素边缘锐利 */
    image-rendering: pixelated;
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
  .plat {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(0, 1fr));
    margin-bottom: 16px;
  }
  .plat button {
    height: 28px;
    font-size: 13px;
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
  @media (max-width: 640px) {
    .wrap {
      height: auto;
      min-height: 60dvh;
      padding: 0 16px 16px;
    }
    .pane:has(.me) {
      justify-content: flex-start;
    }
    .plat {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(0, 1fr));
      margin-bottom: 18px;
    }
    .plat button {
      height: 34px;
      font-size: 14px;
    }
    .seg button {
      height: 34px;
      font-size: 14px;
    }
    .qr {
      width: 220px;
      height: 220px;
    }
    .cookie p,
    .qr-wrap p {
      font-size: 14px;
    }
    textarea.input {
      font-size: 14px;
    }
  }
</style>
