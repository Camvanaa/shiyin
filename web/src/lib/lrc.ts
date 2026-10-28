export interface LrcLine {
  t: number; // 毫秒
  text: string;
  trans?: string;
}

const STAMP = /\[(\d{1,3})[:.](\d{1,2})(?:[:.](\d{1,3}))?[^\]]*\]/g;

function parse(lrc: string): { t: number; text: string }[] {
  const out: { t: number; text: string }[] = [];
  for (const raw of lrc.split('\n')) {
    const stamps: number[] = [];
    let last = 0;
    STAMP.lastIndex = 0;
    let m: RegExpExecArray | null;
    while ((m = STAMP.exec(raw)) && m.index === last) {
      const frac = (m[3] ?? '0').padEnd(3, '0').slice(0, 3);
      stamps.push(+m[1] * 60000 + +m[2] * 1000 + +frac);
      last = STAMP.lastIndex;
    }
    if (!stamps.length) continue;
    const text = raw.slice(last).trim();
    for (const t of stamps) out.push({ t, text });
  }
  return out.sort((a, b) => a.t - b.t);
}

/** 解析原文与翻译，过滤空行，作词作曲等信息行保留 */
export function parseLyric(lrc: string, tlrc: string): LrcLine[] {
  const main = parse(lrc).filter((l) => l.text);
  if (!tlrc) return main;
  const trans = new Map(parse(tlrc).filter((l) => l.text && l.text !== '//').map((l) => [l.t, l.text]));
  return main.map((l) => ({ ...l, trans: trans.get(l.t) }));
}

/** 二分查找当前行 */
export function lineAt(lines: LrcLine[], ms: number): number {
  let lo = 0;
  let hi = lines.length - 1;
  let ans = -1;
  while (lo <= hi) {
    const mid = (lo + hi) >> 1;
    if (lines[mid].t <= ms) {
      ans = mid;
      lo = mid + 1;
    } else hi = mid - 1;
  }
  return ans;
}
