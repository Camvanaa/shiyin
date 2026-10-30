//! 音源适配器接口与注册表。下载、任务调度、路由只依赖本模块，
//! 不感知具体平台。新增音源 = 实现 `Source` 并注册。

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::api::Netease;
use crate::error::{Error, Result};
use crate::model::{Collection, CollectionKind, HomeFeed, Lyric, Resolved, SongUrl, Track};

pub type BoxFut<'a, T> = Pin<Box<dyn Future<Output = Result<T>> + Send + 'a>>;

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

    pub fn for_track(&self, t: &Track) -> Result<&DynSource> {
        self.get(&t.source).ok_or_else(|| Error::Invalid(format!("未知音源：{}", t.source)))
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
            if let Some(ne) = self.get("netease") {
                if let Some(url) = crate::link::first_url(text) {
                    let full = self.http_expand(url).await?;
                    for s in &self.list {
                        if let Some(r) = s.parse_link(&full) {
                            return Ok(r);
                        }
                    }
                    let _ = ne;
                }
            }
        }
        Ok(Resolved::Query(text.to_owned()))
    }

    async fn http_expand(&self, url: &str) -> Result<String> {
        let client = self.list[0].http();
        let resp = client.get(url).timeout(std::time::Duration::from_secs(10)).send().await?;
        resp.headers()
            .get(reqwest::header::LOCATION)
            .and_then(|l| l.to_str().ok())
            .map(str::to_owned)
            .ok_or_else(|| Error::Invalid("短链接无法解析".into()))
    }

    /// 聚合搜索：并发请求各音源，结果交替穿插（避免单一平台占满前排）。
    pub async fn search_all(&self, keywords: &str, limit: u32) -> Vec<Track> {
        let per = (limit / self.list.len().max(1) as u32).max(8);
        let lists = futures_util::future::join_all(
            self.list.iter().map(|s| async move { s.search(keywords, per, 0).await.unwrap_or_default() }),
        )
        .await;
        interleave(lists)
    }

    /// 首页内容：`source` 为 "all" 或空时聚合所有音源，否则只取指定音源。
    pub async fn home(&self, source: &str) -> Result<HomeFeed> {
        if !source.is_empty() && source != "all" {
            let s = self.get(source).ok_or_else(|| Error::Invalid(format!("未知音源：{source}")))?;
            return s.home().await;
        }
        let feeds = futures_util::future::join_all(
            self.list.iter().map(|s| async move { s.home().await.unwrap_or_default() }),
        )
        .await;
        Ok(HomeFeed {
            playlists: interleave(feeds.iter().map(|f| f.playlists.clone()).collect()),
            charts: interleave(feeds.iter().map(|f| f.charts.clone()).collect()),
            new_songs: interleave(feeds.into_iter().map(|f| f.new_songs).collect()),
        })
    }

    /// 在其它音源中按“歌名 + 歌手”寻找可播放的替代曲目（同音源失败时的换源兜底）。
    pub async fn find_playable(&self, name: &str, artist: &str, skip_source: &str, level: &str) -> Option<(Track, SongUrl)> {
        let q = format!("{name} {artist}").trim().to_owned();
        for s in &self.list {
            if s.id() == skip_source {
                continue;
            }
            let Ok(candidates) = s.search(&q, 5, 0).await else { continue };
            for t in candidates.into_iter().filter(|t| t.availability != crate::model::Availability::None) {
                if let Ok(u) = s.song_url(&t.id, level).await {
                    if u.trial.is_none() {
                        return Some((t, u));
                    }
                }
            }
        }
        None
    }
}

fn interleave<T>(lists: Vec<Vec<T>>) -> Vec<T> {
    let max = lists.iter().map(Vec::len).max().unwrap_or(0);
    let mut out = Vec::new();
    for i in 0..max {
        for l in &lists {
            if let Some(v) = l.get(i) {
                // safety: index valid; clone via index move below
                let _ = v;
            }
        }
    }
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
