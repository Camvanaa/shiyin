export function duration(ms: number): string {
  const s = Math.max(0, Math.floor(ms / 1000));
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}`;
}

export function bytes(n: number): string {
  if (n < 1024) return `${n} B`;
  const u = ['KB', 'MB', 'GB'];
  let i = -1;
  do {
    n /= 1024;
    i++;
  } while (n >= 1024 && i < u.length - 1);
  return `${n.toFixed(n < 10 ? 1 : 0)} ${u[i]}`;
}

export function artists(t: { artists: { name: string }[] }): string {
  return t.artists.map((a) => a.name).join(' / ');
}

/** 封面按显示尺寸裁剪（各平台用各自的尺寸参数；无法裁剪的返回原图） */
export function img(url: string, css: number): string {
  if (!url) return '';
  const dpr = Math.min(window.devicePixelRatio || 1, 2);
  const want = css * dpr;

  // 网易云：?param=NyN
  if (url.includes('126.net')) {
    const px = [64, 100, 160, 240, 400, 640, 800, 1024].find((s) => s >= want) ?? 1024;
    return `${url.split('?')[0]}?param=${px}y${px}`;
  }

  // QQ 音乐：y.gtimg.cn/.../T002R{Px}{Px}M000{mid}.jpg
  // 形如 .../T002R300x300M000003h24VO1pTh6K.jpg，把中间的 {n}x{n} 换掉
  if (url.includes('y.gtimg.cn') || url.includes('y.qq.com')) {
    const px = [150, 300, 500, 800].find((s) => s >= want) ?? 800;
    const out = url.replace(/R\d+x\d+M000/, `R${px}x${px}M000`).replace(/\/(\d+)\?n=1$/, `/${px}?n=1`);
    return out;
  }

  // 酷我：.../albumcover/{size}/... 或 .../star/albumcover/{size}/...
  if (url.includes('kuwo.cn')) {
    const px = [120, 500].find((s) => s >= want) ?? 500;
    return url.replace(/\/albumcover\/\d+\//, `/albumcover/${px}/`);
  }

  // Apple：.../{w}x{h}bb.jpg
  if (url.includes('mzstatic.com')) {
    const px = [60, 120, 200, 400, 600, 1000].find((s) => s >= want) ?? 1000;
    return url.replace(/\/\d+x\d+bb\./, `/${px}x${px}bb.`);
  }

  return url;
}

export function year(ts: number | null): string {
  return ts ? String(new Date(ts).getFullYear()) : '';
}

const SOURCE_NAMES: Record<string, [string, string]> = {
  netease: ['网易云', '网易'],
  qq: ['QQ音乐', 'QQ'],
  kuwo: ['酷我音乐', '酷我'],
  apple: ['Apple Music', 'Apple'],
};

export function sourceName(id: string): string {
  return SOURCE_NAMES[id]?.[0] ?? id;
}

export function sourceShort(id: string): string {
  return SOURCE_NAMES[id]?.[1] ?? id;
}

/** 歌曲在原平台的网页链接 */
export function trackLink(t: { id: string; source: string; albumId?: string }): string {
  switch (t.source) {
    case 'qq':
      return `https://y.qq.com/n/ryqq/songDetail/${t.id}`;
    case 'kuwo':
      return `https://www.kuwo.cn/play_detail/${t.id}`;
    case 'apple':
      // 不带地区前缀，Apple 会按访问者所在地区跳转
      return t.albumId ? `https://music.apple.com/album/${t.albumId}?i=${t.id}` : `https://music.apple.com/song/${t.id}`;
    default:
      return `https://music.163.com/song?id=${t.id}`;
  }
}

/** 识别粘贴内容是否为支持的平台链接 */
export const LINK_RE = /music\.163\.com|163cn\.tv|y\.qq\.com|c\.y\.qq\.com|kuwo\.cn|music\.apple\.com/;

export function collectionKindLabel(kind: 'playlist' | 'album' | 'toplist'): string {
  return kind === 'album' ? '专辑' : kind === 'toplist' ? '排行榜' : '歌单';
}
