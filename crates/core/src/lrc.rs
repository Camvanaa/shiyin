//! LRC 处理：时间戳规范化、原文与翻译合并。规则与上游 `lrctran` 一致：
//! 时间相同的翻译行以 ` (翻译：…)` 追加在原文后。

#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    pub ms: u64,
    pub text: String,
}

fn parse_ts(tag: &str) -> Option<u64> {
    // tag 不含方括号，例如 "01:02.34"、"1:2:345"
    let mut parts = tag.split([':', '.']);
    let m: u64 = parts.next()?.trim().parse().ok()?;
    let s: u64 = parts.next()?.trim().parse().ok()?;
    let frac = parts.next().unwrap_or("0");
    let frac: String = frac.chars().take_while(|c| c.is_ascii_digit()).take(3).collect();
    let ms = if frac.is_empty() { 0 } else { format!("{frac:0<3}").parse().ok()? };
    if s >= 60 {
        return None;
    }
    Some(m * 60_000 + s * 1000 + ms)
}

/// 解析为按时间排序的行。一行多个时间戳会展开为多行；元数据行和无时间戳行被丢弃。
pub fn parse(lrc: &str) -> Vec<Line> {
    let mut out = Vec::new();
    for raw in lrc.lines() {
        let mut rest = raw.trim();
        let mut stamps = Vec::new();
        while let Some(r) = rest.strip_prefix('[') {
            let Some(end) = r.find(']') else { break };
            match parse_ts(&r[..end]) {
                Some(ms) => stamps.push(ms),
                None => break,
            }
            rest = r[end + 1..].trim_start();
        }
        if stamps.is_empty() {
            continue;
        }
        let text = rest.split_whitespace().collect::<Vec<_>>().join(" ");
        for ms in stamps {
            out.push(Line { ms, text: text.clone() });
        }
    }
    out.sort_by_key(|l| l.ms);
    out
}

fn fmt_ts(ms: u64) -> String {
    format!("[{:02}:{:02}.{:03}]", ms / 60_000, ms % 60_000 / 1000, ms % 1000)
}

/// 合并原文与翻译，输出规范化 LRC，并保证时间戳严格递增（部分播放器要求）。
pub fn merge(lrc: &str, tlrc: &str) -> String {
    let mut lines = parse(lrc);
    if !tlrc.trim().is_empty() {
        let trans = parse(tlrc);
        let mut j = 0;
        for l in &mut lines {
            while j + 1 < trans.len() && trans[j].ms < l.ms {
                j += 1;
            }
            if let Some(t) = trans.get(j) {
                if t.ms == l.ms {
                    let tt = t.text.replace('/', "");
                    if !tt.trim().is_empty() {
                        l.text.push_str(&format!(" (翻译：{})", tt.trim()));
                    }
                    j += 1;
                }
            }
        }
    }
    let mut prev: Option<u64> = None;
    let mut out = String::new();
    for l in lines {
        let mut ms = l.ms;
        if let Some(p) = prev {
            if ms <= p {
                ms = (l.ms / 1000 * 1000 + 999).min(p + 5);
            }
        }
        prev = Some(ms);
        out.push_str(&fmt_ts(ms));
        out.push_str(&l.text);
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merges_translation() {
        let lrc = "[ti:x]\n[00:01.00]Hello\n[00:02.5][00:04.000]World\n";
        let tlrc = "[00:01.000]你好\n[00:02.500]世界\n";
        assert_eq!(
            merge(lrc, tlrc),
            "[00:01.000]Hello (翻译：你好)\n[00:02.500]World (翻译：世界)\n[00:04.000]World\n"
        );
    }

    #[test]
    fn resolves_duplicate_stamps() {
        assert_eq!(merge("[00:01.00]a\n[00:01.00]b", ""), "[00:01.000]a\n[00:01.005]b\n");
    }
}
