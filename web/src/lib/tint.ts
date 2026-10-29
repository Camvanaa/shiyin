import { img } from './format';

const cache = new Map<string, string>();

/** 从封面提取主色，返回 "r g b"。失败时返回空字符串。 */
export function extractTint(url: string): Promise<string> {
  if (!url) return Promise.resolve('');
  const hit = cache.get(url);
  if (hit !== undefined) return Promise.resolve(hit);
  return new Promise((resolve) => {
    const im = new Image();
    im.crossOrigin = 'anonymous';
    im.decoding = 'async';
    im.src = img(url, 32);
    im.onerror = () => resolve('');
    im.onload = () => {
      try {
        const c = document.createElement('canvas');
        c.width = c.height = 12;
        const g = c.getContext('2d', { willReadFrequently: true })!;
        g.drawImage(im, 0, 0, 12, 12);
        const d = g.getImageData(0, 0, 12, 12).data;
        // 按饱和度加权取主色，避免平均成灰色
        let r = 0, gg = 0, b = 0, wsum = 0;
        for (let i = 0; i < d.length; i += 4) {
          const R = d[i], G = d[i + 1], B = d[i + 2];
          const mx = Math.max(R, G, B), mn = Math.min(R, G, B);
          const w = 0.08 + (mx - mn) / 255 + (mx > 40 && mx < 235 ? 0.3 : 0);
          r += R * w;
          gg += G * w;
          b += B * w;
          wsum += w;
        }
        r /= wsum;
        gg /= wsum;
        b /= wsum;
        const lum = 0.299 * r + 0.587 * gg + 0.114 * b;
        const k = lum < 90 ? 90 / Math.max(lum, 1) : lum > 170 ? 170 / lum : 1;
        const ch = (v: number) => Math.min(255, Math.round(v * k));
        const out = `${ch(r)} ${ch(gg)} ${ch(b)}`;
        cache.set(url, out);
        resolve(out);
      } catch {
        resolve('');
      }
    };
  });
}
