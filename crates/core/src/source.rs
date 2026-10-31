//! 音源适配器接口与注册表。下载、任务调度、路由只依赖本模块，
//! 不感知具体平台。新增音源 = 实现 `Source` 并注册。

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use crate::api::Netease;
use crate::error::{Error, Result};
use crate::model::{Collection, CollectionKind, HomeFeed, Lyric, Resolved, SongUrl, Track};

pub type BoxFut<'a, T> = Pin<Box<dyn Future<Output = Result<T>> + Send + 'a>>;

/// 聚合请求时单个音源的最长等待时间。超时的音源结果被丢弃，不拖慢整体响应。
const AGGREGATE_TIMEOUT: Duration = Duration::from_secs(8);
/// 换源候选的播放地址校验超时，比聚合更短，避免逐个候选拖长面板等待。
const CANDIDATE_TIMEOUT: Duration = Duration::from_secs(5);

pub trait Source: Send + Sync {
    fn id(&self) -> &'static str;
    fn label(&self) -> &'static str;
    /// 支持的音质键（standard/exhigh/lossless/hires/...）
    fn levels(&self) -> &'static [&'static str];

    fn http(&self) -> &reqwest::Client;

    /// 识别用户粘贴的链接或 ID，不识别返回 None（同步、不联网）
    fn parse_link(&self, text: &str) -> Option<Resolved>;

    fn search<'a>(&'a self, keywords: &'a str, limit: u32, offset: u32) -> BoxFut<'a, Vec<Track>>;
    fn track<'a>(&'a self, id: &'a str) -> BoxFut<'a, Track>;
    fn collection(&self, kind: CollectionKind, id: &str) -> BoxFut<'_, Collection>;
    fn song_url<'a>(&'a self, id: &'a str, level: &'a str) -> BoxFut<'a, SongUrl>;
    fn lyric<'a>(&'a self, id: &'a str) -> BoxFut<'a, Lyric>;

    /// 首页推荐内容。默认为空。
    fn home(&self) -> BoxFut<'_, HomeFeed> {
        Box::pin(async { Ok(HomeFeed::default()) })
    }

    /// 设置平台 Cookie（内存态），返回是否支持。
    fn set_cookie(&self, _cookie: Option<String>) {}
    fn has_cookie(&self) -> bool {
        false
    }

    /// 按显示尺寸返回封面地址，默认原样返回
    fn sized_cover(&self, url: &str, _size: u32) -> String {
        url.to_owned()
    }
    /// 媒体地址域名白名单，防止服务端被当作开放代理
    fn is_allowed_media_url(&self, url: &str) -> bool;
}

pub type DynSource = Arc<dyn Source>;

#[derive(Clone)]
pub struct Sources {
    list: Vec<DynSource>,
}

impl Sources {
    pub fn new(netease: Netease) -> Self {
        let list: Vec<DynSource> = vec![
            Arc::new(netease),
            Arc::new(crate::providers::kuwo::Kuwo::new()),
            Arc::new(crate::providers::qq::QQ::new()),
        ];
        Self { list }
    }

    pub fn list(&self) -> &[DynSource] {
        &self.list
    }

    pub fn get(&self, id: &str) -> Option<&DynSource> {
        self.list.iter().find(|s| s.id() == id)
    }

    /// 按 ID 取音源；空字符串回落到网易云。
    pub fn pick(&self, id: &str) -> Result<&DynSource> {
        let key = if id.is_empty() { "netease" } else { id };
        self.get(key).ok_or_else(|| Error::Invalid(format!("未知音源：{key}")))
    }

    pub fn for_track(&self, t: &Track) -> Result<&DynSource> {
        self.pick(&t.source)
    }

    /// 从持久化存储和环境变量恢复各音源凭证。
    /// 网易云由调用方单独处理（走 `Netease::set_music_u`）。
    pub fn restore_cookies(&self, load: impl Fn(&str) -> Option<String>) {
        for src in &self.list {
            if src.id() == "netease" {
                continue;
            }
            let env_key = format!("{}_COOKIE", src.id().to_ascii_uppercase());
            src.set_cookie(load(src.id()).or_else(|| std::env::var(env_key).ok()));
        }
    }

    /// 依次让各音源识别链接；网易短链单独展开
    pub async fn resolve(&self, text: &str) -> Result<Resolved> {
        let text = text.trim();
        for s in &self.list {
            if let Some(r) = s.parse_link(text) {
                return Ok(r);
            }
        }
        // 163cn.tv 短链需要联网展开后再次识别
        if text.contains("163cn.tv") {
            if let Some(url) = crate::link::first_url(text) {
                let full = self.expand_short_link(url).await?;
                for s in &self.list {
                    if let Some(r) = s.parse_link(&full) {
                        return Ok(r);
                    }
                }
            }
        }
        Ok(Resolved::Query { value: text.to_owned() })
    }

    async fn expand_short_link(&self, url: &str) -> Result<String> {
        let client = self.list[0].http();
        let resp = client.get(url).timeout(Duration::from_secs(10)).send().await?;
        resp.headers()
            .get(reqwest::header::LOCATION)
            .and_then(|l| l.to_str().ok())
            .map(str::to_owned)
            .ok_or_else(|| Error::Invalid("短链接无法解析".into()))
    }

    /// 聚合搜索：并发请求各音源，结果交替穿插（避免单一平台占满前排）。
    /// `offset` 为聚合层偏移，按音源数均分后传给各平台，实现"加载更多"。
    pub async fn search_all(&self, keywords: &str, limit: u32, offset: u32) -> Vec<Track> {
        let n = self.list.len().max(1) as u32;
        let per = (limit / n).max(8);
        let per_offset = offset / n;
        let lists = futures_util::future::join_all(self.list.iter().map(|s| async move {
            tokio::time::timeout(AGGREGATE_TIMEOUT, s.search(keywords, per, per_offset))
                .await
                .ok()
                .and_then(Result::ok)
                .unwrap_or_default()
        }))
        .await;
        interleave(lists)
    }

    /// 首页内容：`source` 为 "all" 或空时聚合所有音源，否则只取指定音源。
    pub async fn home(&self, source: &str) -> Result<HomeFeed> {
        if !source.is_empty() && source != "all" {
            return self.pick(source)?.home().await;
        }
        let feeds = futures_util::future::join_all(self.list.iter().map(|s| async move {
            tokio::time::timeout(AGGREGATE_TIMEOUT, s.home())
                .await
                .ok()
                .and_then(Result::ok)
                .unwrap_or_default()
        }))
        .await;
        let mut playlists = Vec::new();
        let mut charts = Vec::new();
        let mut new_songs = Vec::new();
        for f in feeds {
            playlists.push(f.playlists);
            charts.push(f.charts);
            new_songs.push(f.new_songs);
        }
        Ok(HomeFeed {
            playlists: interleave(playlists),
            charts: interleave(charts),
            new_songs: interleave(new_songs),
        })
    }

    /// 在其它音源中按"歌名 + 歌手"寻找可播放的替代曲目（同音源失败时的换源兜底）。
    /// 各音源并发查找，取第一个成功的结果。
    pub async fn find_playable(&self, name: &str, artist: &str, duration_ms: u64, skip_source: &str, level: &str) -> Option<(Track, SongUrl)> {
        // 自动换源只接受时长相近且歌名匹配的版本，避免悄悄换成 Live / 翻唱
        self.find_candidates_for(name, artist, duration_ms, skip_source, level)
            .await
            .into_iter()
            .find(|(t, _)| duration_ms == 0 || t.duration == 0 || t.duration.abs_diff(duration_ms) as f64 / duration_ms as f64 <= 0.12)
    }

    /// 在其它音源中列出所有可完整播放的同名版本，按匹配度排序（供用户手动换源）。
    pub async fn find_candidates(&self, name: &str, artist: &str, skip_source: &str, level: &str) -> Vec<(Track, SongUrl)> {
        self.find_candidates_for(name, artist, 0, skip_source, level).await
    }

    /// 同上，可传入原曲时长（毫秒）用于过滤 Live / 混剪 / 片段等时长差异过大的版本；0 表示不过滤。
    pub async fn find_candidates_for(&self, name: &str, artist: &str, duration_ms: u64, skip_source: &str, level: &str) -> Vec<(Track, SongUrl)> {
        let q = format!("{name} {artist}").trim().to_owned();
        let name_lc = name.trim().to_lowercase();
        let artist_lc = artist.trim().to_lowercase();
        let tasks = self.list.iter().filter(|s| s.id() != skip_source).map(|s| {
            let q = q.clone();
            let name_lc = name_lc.clone();
            let artist_lc = artist_lc.clone();
            async move {
                let candidates = tokio::time::timeout(AGGREGATE_TIMEOUT, s.search(&q, 6, 0)).await.ok()?.ok()?;
                let mut ranked: Vec<(u8, Track)> = candidates
                    .into_iter()
                    .filter(|t| t.availability != crate::model::Availability::None)
                    .map(|t| {
                        let n = t.name.trim().to_lowercase();
                        let a = t.artist_names().to_lowercase();
                        // 0 = 歌名歌手都匹配；1 = 仅歌名；2 = 其它。
                        // 时长相差超过 12% 视为不同版本（Live / 混剪 / 翻唱），降权到最后。
                        let mut score = if n == name_lc && (artist_lc.is_empty() || a.contains(&artist_lc)) { 0 } else if n == name_lc { 1 } else { 2 };
                        if duration_ms > 0 && t.duration > 0 {
                            let diff = t.duration.abs_diff(duration_ms) as f64 / duration_ms as f64;
                            if diff > 0.12 {
                                score += 3;
                            }
                        }
                        (score, t)
                    })
                    .collect();
                ranked.sort_by_key(|(s, _)| *s);
                // 每个音源最多校验前 3 个候选，并发请求，取排序最靠前的可用版本
                let picks: Vec<(u8, Track)> = ranked.into_iter().take(3).collect();
                let order: Vec<u8> = picks.iter().map(|(s, _)| *s).collect();
                let checks = picks.into_iter().map(|(_, t)| {
                    let id = t.id.clone();
                    async move {
                        let u = tokio::time::timeout(CANDIDATE_TIMEOUT, s.song_url(&id, level)).await.ok()?.ok()?;
                        if u.trial.is_none() && u.url.starts_with("http") {
                            Some((t, u))
                        } else {
                            None
                        }
                    }
                });
                let results = futures_util::future::join_all(checks).await;
                // 保持原始排序：score 越小越优先
                let mut best: Option<(u8, Track, SongUrl)> = None;
                for (score, r) in order.into_iter().zip(results) {
                    if let Some((t, u)) = r {
                        if best.as_ref().is_none_or(|(bs, _, _)| score < *bs) {
                            best = Some((score, t, u));
                        }
                    }
                }
                best
            }
        });
        let mut found: Vec<(u8, Track, SongUrl)> = futures_util::future::join_all(tasks).await.into_iter().flatten().collect();
        found.sort_by_key(|(s, _, _)| *s);
        found.into_iter().map(|(_, t, u)| (t, u)).collect()
    }
}

/// 多个列表交替穿插：a0 b0 c0 a1 b1 c1 ...
fn interleave<T>(lists: Vec<Vec<T>>) -> Vec<T> {
    let total: usize = lists.iter().map(Vec::len).sum();
    let max = lists.iter().map(Vec::len).max().unwrap_or(0);
    let mut out = Vec::with_capacity(total);
    let mut iters: Vec<_> = lists.into_iter().map(Vec::into_iter).collect();
    for _ in 0..max {
        for it in &mut iters {
            if let Some(v) = it.next() {
                out.push(v);
            }
        }
    }
    out
}

// ---------------- 网易云适配 ----------------

fn to_u64(id: &str) -> Result<u64> {
    id.trim().parse().map_err(|_| Error::Invalid(format!("网易云歌曲 ID 无效：{id}")))
}

impl Source for Netease {
    fn id(&self) -> &'static str {
        "netease"
    }
    fn label(&self) -> &'static str {
        "网易云音乐"
    }
    fn levels(&self) -> &'static [&'static str] {
        &["standard", "exhigh", "lossless", "hires", "sky", "jyeffect", "jymaster"]
    }
    fn http(&self) -> &reqwest::Client {
        Netease::http(self)
    }
    fn parse_link(&self, text: &str) -> Option<Resolved> {
        crate::link::classify(text).ok()
    }
    fn search<'a>(&'a self, keywords: &'a str, limit: u32, offset: u32) -> BoxFut<'a, Vec<Track>> {
        Box::pin(Netease::search(self, keywords, limit, offset))
    }
    fn track<'a>(&'a self, id: &'a str) -> BoxFut<'a, Track> {
        Box::pin(async move {
            let n = to_u64(id)?;
            self.song_details(&[n]).await?.pop().ok_or_else(|| Error::Invalid("歌曲不存在".into()))
        })
    }
    fn collection(&self, kind: CollectionKind, id: &str) -> BoxFut<'_, Collection> {
        let id = id.to_string();
        Box::pin(async move {
            let n = to_u64(&id)?;
            match kind {
                CollectionKind::Playlist | CollectionKind::Toplist => self.playlist(n).await,
                CollectionKind::Album => self.album(n).await,
            }
        })
    }
    fn song_url<'a>(&'a self, id: &'a str, level: &'a str) -> BoxFut<'a, SongUrl> {
        Box::pin(async move { Netease::song_url(self, to_u64(id)?, level).await })
    }
    fn lyric<'a>(&'a self, id: &'a str) -> BoxFut<'a, Lyric> {
        Box::pin(async move { Netease::lyric(self, to_u64(id)?).await })
    }
    fn home(&self) -> BoxFut<'_, HomeFeed> {
        Box::pin(Netease::home(self))
    }
    fn set_cookie(&self, cookie: Option<String>) {
        self.set_music_u(cookie);
    }
    fn has_cookie(&self) -> bool {
        self.music_u().is_some()
    }
    fn sized_cover(&self, url: &str, size: u32) -> String {
        crate::download::sized_cover(url, size)
    }
    fn is_allowed_media_url(&self, url: &str) -> bool {
        crate::download::is_allowed_media_url(url)
    }
}
