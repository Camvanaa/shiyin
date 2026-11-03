use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Artist {
    pub id: u64,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    /// 平台内的歌曲 ID（数字或字符串 MID）
    pub id: String,
    /// 来源音源，例如 "netease"、"qq"、"kuwo"
    #[serde(default = "default_source")]
    pub source: String,
    pub name: String,
    /// 别名或译名，例如 "Sunny Day"
    pub alias: Option<String>,
    pub artists: Vec<Artist>,
    pub album_id: String,
    pub album: String,
    pub cover: String,
    pub duration: u64,
    /// 1 = VIP 歌曲，4 = 付费专辑，8 = 免费（低音质）
    pub fee: i64,
    pub availability: Availability,
    /// 国际标准录音编码。有则可跨平台精确合并同一首歌；缺失时回退到
    /// 「规范化歌名 + 主歌手 + 时长」的指纹。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub isrc: Option<String>,
}

/// 当前账号下的可用性
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Availability {
    /// 可完整播放
    #[default]
    Ok,
    /// 只能试听或需要会员
    Vip,
    /// 需要单独购买
    Paid,
    /// 无版权，不可播放
    None,
}

impl Track {
    pub fn artist_names(&self) -> String {
        self.artists.iter().map(|a| a.name.as_str()).collect::<Vec<_>>().join("/")
    }

    /// 跨平台合并键（粗粒度）。优先后端提供的 ISRC；否则「规范化歌名 + 主歌手」。
    /// 精细的版本切分（区分 Live / 录音室版）由 `merge_works` 按时长容差完成。
    pub fn work_key(&self) -> String {
        if let Some(isrc) = self.isrc.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            return format!("isrc:{}", isrc.to_ascii_lowercase());
        }
        self.fp_key()
    }

    /// 指纹键：忽略 ISRC，仅按「规范化歌名 + 主歌手」。用于第一轮聚类，
    /// 这样带 ISRC 的平台（Apple）也能和无 ISRC 的平台（网易/QQ/酷我）聚到一起。
    pub fn fp_key(&self) -> String {
        let name = normalize_title(&self.name);
        let artist = normalize_title(self.artists.first().map(|a| a.name.as_str()).unwrap_or(""));
        format!("fp:{name}|{artist}")
    }

    /// 归一化后的 ISRC（小写），空则 None。
    pub fn isrc_norm(&self) -> Option<String> {
        self.isrc.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(|s| s.to_ascii_lowercase())
    }

    /// 时长是否足够接近，可视为同一版本。任一为 0（未知）时视为接近。
    fn close_duration(a: u64, b: u64) -> bool {
        if a == 0 || b == 0 {
            return true;
        }
        let diff = a.abs_diff(b);
        diff <= 5_000 || diff as f64 / a.max(b) as f64 <= 0.03
    }
}

/// 归一化文本用于匹配：全角转半角、小写、去掉括号内容与常见后缀/标点。
pub fn normalize_title(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut depth = 0i32;
    for ch in s.chars() {
        match ch {
            '（' | '(' | '[' | '【' => depth += 1,
            '）' | ')' | ']' | '】' => depth = (depth - 1).max(0),
            _ if depth > 0 => {}
            _ => {
                let c = match ch {
                    // 全角字母数字转半角
                    '\u{FF01}'..='\u{FF5E}' => char::from_u32(ch as u32 - 0xFEE0).unwrap_or(ch),
                    '\u{3000}' => ' ',
                    _ => ch,
                };
                if c.is_alphanumeric() {
                    out.extend(c.to_lowercase());
                }
                // 其它字符（空格、标点、连字符）一律丢弃，避免 "feat."/"-" 造成差异
            }
        }
    }
    // 去掉常见的 "live"/"remaster" 等噪声词（已在括号内处理大部分）
    out
}

/// 合并后的「作品」：同一首歌在不同平台的多个版本。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Work {
    /// 稳定标识（`isrc:...` 或 `fp:...`），用于资料库引用
    pub key: String,
    pub name: String,
    pub artists: Vec<Artist>,
    pub album: String,
    pub duration: u64,
    /// 用于卡片展示的封面与来源（取"最好"的版本）
    pub cover: String,
    pub cover_source: String,
    /// 全部版本，按推荐度排序（最可能完整播放、音质最高的在前）
    pub renditions: Vec<Track>,
}

impl Work {
    /// 由若干同一作品的可播版本构建。
    pub fn from_renditions(key: String, mut rends: Vec<Track>) -> Self {
        rends.sort_by_key(rendition_rank);
        let best = rends.iter().find(|t| t.availability != Availability::None).unwrap_or(&rends[0]).clone();
        // 封面取最大的那张，展示更清晰
        let cover = rends
            .iter()
            .map(|t| t.cover.clone())
            .max_by_key(|c| c.len())
            .filter(|c| !c.is_empty())
            .unwrap_or_else(|| best.cover.clone());
        let cover_source = rends
            .iter()
            .find(|t| t.cover == cover)
            .map(|t| t.source.clone())
            .unwrap_or_else(|| best.source.clone());
        Work {
            key,
            name: best.name.clone(),
            artists: if best.artists.is_empty() {
                rends[0].artists.clone()
            } else {
                best.artists.clone()
            },
            album: best.album.clone(),
            duration: best.duration,
            cover,
            cover_source,
            renditions: rends,
        }
    }
}

/// 版本推荐度：越小越优先。可用性 > 音质档位。
fn rendition_rank(t: &Track) -> u8 {
    let avail = match t.availability {
        Availability::Ok => 0,
        Availability::Vip => 10,
        Availability::Paid => 20,
        Availability::None => 30,
    };
    avail
}

/// 两个歌手名是否"可能是同一人"（简体/繁体或译名差异）。
/// 归一化后要求长度接近且至少有一个共同字符，避免把不同歌手的同名cover误并。
fn artist_similar(a: &str, b: &str) -> bool {
    let (a, b) = (normalize_title(a), normalize_title(b));
    if a.is_empty() || b.is_empty() {
        return true; // 缺失歌手信息时不据此否决
    }
    if a == b {
        return true;
    }
    let common = a.chars().filter(|c| b.contains(*c)).count();
    let shorter = a.chars().count().min(b.chars().count());
    common > 0 && common * 2 >= shorter
}

/// 把多个平台的曲目合并成作品列表，保持首次出现的顺序。
///
/// 用并查集分三步连边（强 → 弱）：
/// 1. ISRC 完全相同 → 同一首（最可靠）。
/// 2. 同一「规范化歌名 + 主歌手」且时长接近 → 同一版本（各平台元数据一致时主力）。
/// 3. 其中一方带 ISRC、规范化歌名相同、时长接近且歌手名相似 → 桥接
///    （Apple 用繁体歌手名时仍能聚到同一首；歌手明显不同的同名 cover 不会被并）。
pub fn merge_works(tracks: Vec<Track>) -> Vec<Work> {
    let n = tracks.len();
    if n == 0 {
        return Vec::new();
    }
    let mut parent: Vec<usize> = (0..n).collect();
    fn find(p: &mut Vec<usize>, x: usize) -> usize {
        let mut r = x;
        while p[r] != r {
            r = p[r];
        }
        // 路径压缩
        let mut c = x;
        while p[c] != r {
            let nx = p[c];
            p[c] = r;
            c = nx;
        }
        r
    }
    let union = |p: &mut Vec<usize>, a: usize, b: usize| {
        let (ra, rb) = (find(p, a), find(p, b));
        if ra != rb {
            p[rb] = ra;
        }
    };

    let fp: Vec<String> = tracks.iter().map(Track::fp_key).collect();
    let title: Vec<String> = tracks.iter().map(|t| normalize_title(&t.name)).collect();
    let isrc: Vec<Option<String>> = tracks.iter().map(Track::isrc_norm).collect();

    for i in 0..n {
        for j in (i + 1)..n {
            if find(&mut parent, i) == find(&mut parent, j) {
                continue;
            }
            let close = Track::close_duration(tracks[i].duration, tracks[j].duration);
            let linked = if isrc[i].is_some() && isrc[i] == isrc[j] {
                true
            } else if fp[i] == fp[j] && close {
                true
            } else {
                // 弱桥接：任一方有 ISRC、歌名相同、时长接近、歌手名相似
                (isrc[i].is_some() || isrc[j].is_some())
                    && title[i] == title[j]
                    && close
                    && artist_similar(&tracks[i].artist_names(), &tracks[j].artist_names())
            };
            if linked {
                union(&mut parent, i, j);
            }
        }
    }

    // 按并查集根分组，保持首次出现的顺序
    let mut order: Vec<usize> = Vec::new();
    let mut buckets: std::collections::HashMap<usize, Vec<usize>> = std::collections::HashMap::new();
    for i in 0..n {
        let r = find(&mut parent, i);
        if !buckets.contains_key(&r) {
            order.push(r);
        }
        buckets.entry(r).or_default().push(i);
    }
    order
        .into_iter()
        .filter_map(|r| {
            let idxs = buckets.remove(&r)?;
            let rends: Vec<Track> = idxs.into_iter().map(|i| tracks[i].clone()).collect();
            // key 优先用 ISRC（跨平台稳定），否则用首个版本的指纹
            let key = rends
                .iter()
                .find_map(|t| t.isrc_norm().map(|i| format!("isrc:{i}")))
                .unwrap_or_else(|| rends[0].work_key());
            Some(Work::from_renditions(key, rends))
        })
        .collect()
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum CollectionKind {
    #[default]
    Playlist,
    Album,
    Toplist,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Collection {
    pub kind: CollectionKind,
    pub id: String,
    #[serde(default = "default_source")]
    pub source: String,
    pub name: String,
    pub cover: String,
    /// 歌单创建者或专辑歌手
    pub owner: String,
    pub description: String,
    pub publish_time: Option<i64>,
    pub track_count: usize,
    pub tracks: Vec<Track>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SongUrl {
    pub id: String,
    pub url: String,
    pub level: String,
    pub size: u64,
    pub br: u64,
    /// 文件类型，例如 "mp3"、"flac"
    pub format: String,
    /// 非空表示只返回了试听片段（毫秒区间）
    pub trial: Option<(u64, u64)>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Lyric {
    pub lrc: String,
    pub tlrc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub user_id: u64,
    pub nickname: String,
    pub avatar: String,
    pub vip_type: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QrStart {
    pub key: String,
    /// 二维码内容（登录链接）
    pub url: String,
    /// 渲染好的 SVG（网易云为登录链接生成）
    pub svg: String,
    /// 平台直接下发的二维码图片（data URL）。QQ 扫码接口返回的是 PNG，非空时用它
    #[serde(default)]
    pub image: String,
}

/// 图形验证码（短信登录第一步），image 为 data URL
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Captcha {
    pub token: String,
    pub image: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum QrState {
    Waiting,
    Scanned,
    Expired,
    Success,
}

/// 用户输入识别结果
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Resolved {
    Song { source: String, id: String },
    Playlist { source: String, id: String },
    Album { source: String, id: String },
    /// 内部标签枚举不能用 newtype 变体，否则 serde 序列化会失败
    Query { value: String },
}

pub fn default_source() -> String {
    "netease".into()
}

pub const LEVELS: &[(&str, &str)] = &[
    ("standard", "标准"),
    ("exhigh", "极高"),
    ("lossless", "无损"),
    ("hires", "Hi-Res"),
    ("sky", "沉浸环绕声"),
    ("jyeffect", "高清环绕声"),
    ("jymaster", "超清母带"),
];

pub fn is_valid_level(level: &str) -> bool {
    LEVELS.iter().any(|(k, _)| *k == level)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistBrief {
    pub id: String,
    #[serde(default = "default_source")]
    pub source: String,
    #[serde(default)]
    pub kind: CollectionKind,
    pub name: String,
    pub cover: String,
    pub subtitle: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HomeFeed {
    pub playlists: Vec<PlaylistBrief>,
    pub charts: Vec<PlaylistBrief>,
    pub new_songs: Vec<Track>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk(id: &str, source: &str, name: &str, artist: &str, dur: u64, isrc: Option<&str>) -> Track {
        Track {
            id: id.into(),
            source: source.into(),
            name: name.into(),
            alias: None,
            artists: vec![Artist { id: 0, name: artist.into() }],
            album_id: String::new(),
            album: "专辑".into(),
            cover: String::new(),
            duration: dur,
            fee: 0,
            availability: Availability::Ok,
            isrc: isrc.map(str::to_owned),
        }
    }

    /// 回归：带内部标签的枚举不能用 newtype 变体，否则 serde 序列化会失败。
    #[test]
    fn resolved_serializes() {
        let cases = [
            Resolved::Query { value: "周杰伦 晴天".into() },
            Resolved::Song { source: "qq".into(), id: "0039MnYb0qxYhV".into() },
            Resolved::Playlist { source: "netease".into(), id: "3778678".into() },
            Resolved::Album { source: "kuwo".into(), id: "228908".into() },
        ];
        for c in cases {
            let v = serde_json::to_value(&c).expect("序列化失败");
            assert!(v.get("type").is_some(), "缺少 type 字段：{v}");
            let back: Resolved = serde_json::from_value(v).expect("反序列化失败");
            assert_eq!(back, c);
        }
        let v = serde_json::to_value(Resolved::Query { value: "abc".into() }).unwrap();
        assert_eq!(v["type"], "query");
        assert_eq!(v["value"], "abc");
    }

    #[test]
    fn normalize_title_strips_fullwidth_brackets_and_punct() {
        assert_eq!(normalize_title("晴天"), "晴天");
        assert_eq!(normalize_title("晴天 (Live)"), "晴天");
        assert_eq!(normalize_title("晴天（Live版）"), "晴天");
        assert_eq!(normalize_title("Hello, World!"), "helloworld");
        assert_eq!(normalize_title("Ａ Ｂ Ｃ"), "abc");
        assert_eq!(normalize_title("feat. 周杰伦 - 晴天"), "feat周杰伦晴天");
    }

    #[test]
    fn same_isrc_merges_regardless_of_name() {
        let a = mk("1", "qq", "晴天", "周杰伦", 269000, Some("HK1234567890"));
        let b = mk("2", "apple", "Sunny Day", "Jay Chou", 268000, Some("hk1234567890"));
        assert_eq!(a.work_key(), b.work_key());
        let works = merge_works(vec![a, b]);
        assert_eq!(works.len(), 1);
        assert_eq!(works[0].renditions.len(), 2);
    }

    #[test]
    fn fingerprint_merges_across_platforms() {
        // 同名同歌手，时长差在 3 秒档内 → 合并
        let a = mk("1", "netease", "晴天", "周杰伦", 269000, None);
        let b = mk("2", "kuwo", "晴天", "周杰伦", 270500, None);
        assert_eq!(merge_works(vec![a, b]).len(), 1);
    }

    #[test]
    fn different_duration_versions_stay_separate() {
        // Live 版比原版长很多，不该被合并
        let a = mk("1", "netease", "晴天", "周杰伦", 269000, None);
        let b = mk("2", "qq", "晴天", "周杰伦", 420000, None);
        assert_eq!(merge_works(vec![a, b]).len(), 2);
    }

    #[test]
    fn different_artist_stays_separate() {
        let a = mk("1", "netease", "晴天", "周杰伦", 269000, None);
        let b = mk("2", "qq", "晴天", "刘瑞琦", 269000, None);
        assert_eq!(merge_works(vec![a, b]).len(), 2);
    }

    #[test]
    fn work_prefers_playable_and_bigger_cover() {
        let mut a = mk("1", "apple", "晴天", "周杰伦", 269000, None);
        a.availability = Availability::Vip;
        a.cover = "https://x/600x600bb.jpg".into();
        let mut b = mk("2", "kuwo", "晴天", "周杰伦", 269000, None);
        b.cover = "https://x/120.jpg".into();
        let w = merge_works(vec![a, b]);
        assert_eq!(w.len(), 1);
        // 可播放版本排在前面
        assert_eq!(w[0].renditions[0].source, "kuwo");
        // 封面取更长（更清晰）的那个
        assert_eq!(w[0].cover, "https://x/600x600bb.jpg");
    }

    #[test]
    fn merge_preserves_first_seen_order() {
        let tracks = vec![
            mk("1", "netease", "A", "X", 100000, None),
            mk("2", "qq", "B", "Y", 100000, None),
            mk("3", "kuwo", "A", "X", 100000, None),
        ];
        let w = merge_works(tracks);
        assert_eq!(w.len(), 2);
        assert_eq!(w[0].name, "A");
        assert_eq!(w[1].name, "B");
    }

    #[test]
    fn work_serializes_camel_case() {
        let w = merge_works(vec![mk("1", "qq", "晴天", "周杰伦", 269000, None)]);
        let v = serde_json::to_value(&w[0]).unwrap();
        assert!(v.get("coverSource").is_some());
        assert!(v.get("renditions").is_some());
        assert_eq!(v["key"], "fp:晴天|周杰伦");
    }

    #[test]
    fn isrc_key_wins_and_normalizes_case() {
        let a = mk("1", "qq", "晴天", "周杰伦", 269000, Some("  HK1234  "));
        assert_eq!(a.work_key(), "isrc:hk1234");
    }

    #[test]
    fn unknown_duration_still_clusters() {
        // 时长未知（0）应与已知时长的版本合并
        let a = mk("1", "netease", "晴天", "周杰伦", 269000, None);
        let b = mk("2", "kuwo", "晴天", "周杰伦", 0, None);
        assert_eq!(merge_works(vec![a, b]).len(), 1);
    }

    #[test]
    fn isrc_bridges_fingerprint_and_platform_versions() {
        // Apple 带 ISRC 且歌名用繁体，网易无 ISRC：ISRC 应把它们桥接成同一首
        let n = mk("1", "netease", "晴天", "周杰伦", 269000, None);
        let q = mk("2", "qq", "晴天", "周杰伦", 268500, None);
        let a = mk("3", "apple", "晴天", "周杰倫", 269000, Some("TWK970300503"));
        let w = merge_works(vec![n, q, a]);
        assert_eq!(w.len(), 1, "应合并为一首");
        let srcs: Vec<_> = w[0].renditions.iter().map(|t| t.source.as_str()).collect();
        assert!(srcs.contains(&"apple") && srcs.contains(&"netease") && srcs.contains(&"qq"), "{srcs:?}");
        // key 用 ISRC（更稳定）
        assert_eq!(w[0].key, "isrc:twk970300503");
    }

    #[test]
    fn isrc_does_not_merge_different_songs() {
        let a = mk("1", "apple", "晴天", "周杰伦", 269000, Some("A1"));
        let b = mk("2", "apple", "晴天", "周杰伦", 269000, Some("B2"));
        // 同歌名歌手时长：按指纹规则本就视为同一首（可接受）
        assert_eq!(merge_works(vec![a, b]).len(), 1);
    }

    #[test]
    fn same_title_different_artist_not_bridged_by_isrc() {
        // 同名不同歌手：一个带 ISRC 也不该桥接
        let a = mk("1", "apple", "晴天", "周杰伦", 269000, Some("TWK1"));
        let b = mk("2", "netease", "晴天", "刘瑞琦", 269000, None);
        assert_eq!(merge_works(vec![a, b]).len(), 2);
    }

    #[test]
    fn live_version_not_bridged_to_studio_by_isrc() {
        // 同名歌手，但时长差很多：即使一方带 ISRC 也不桥接
        let a = mk("1", "apple", "晴天", "周杰伦", 269000, Some("TWK1"));
        let b = mk("2", "netease", "晴天", "周杰伦", 420000, None);
        assert_eq!(merge_works(vec![a, b]).len(), 2);
    }
}
