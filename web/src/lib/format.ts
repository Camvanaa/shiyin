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

/** 封面按显示尺寸裁剪（仅网易云支持 ?param=，其他平台用原图） */
export function img(url: string, css: number): string {
  if (!url) return '';
  if (!url.includes('126.net')) return url;
  const dpr = Math.min(window.devicePixelRatio || 1, 2);
  const px = [64, 100, 160, 240, 400, 640, 800].find((s) => s >= css * dpr) ?? 800;
  return `${url.split('?')[0]}?param=${px}y${px}`;
}

export function year(ts: number | null): string {
  return ts ? String(new Date(ts).getFullYear()) : '';
}

export function sourceName(id: string): string {
  return id === 'qq' ? 'QQ音乐' : id === 'kuwo' ? '酷我音乐' : id === 'netease' ? '网易云' : id;
}

export function sourceShort(id: string): string {
  return id === 'qq' ? 'QQ' : id === 'kuwo' ? '酷我' : '网易';
}

/** 歌曲在原平台的网页链接 */
export function trackLink(t: { id: string; source: string }): string {
  switch (t.source) {
    case 'qq':
      return `https://y.qq.com/n/ryqq/songDetail/${t.id}`;
    case 'kuwo':
      return `https://www.kuwo.cn/play_detail/${t.id}`;
    default:
      return `https://music.163.com/song?id=${t.id}`;
  }
}

/** 识别粘贴内容是否为支持的平台链接 */
export const LINK_RE = /music\.163\.com|163cn\.tv|y\.qq\.com|c\.y\.qq\.com|kuwo\.cn/;

export function collectionKindLabel(kind: 'playlist' | 'album' | 'toplist'): string {
  return kind === 'album' ? '专辑' : kind === 'toplist' ? '排行榜' : '歌单';
}
