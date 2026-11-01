//! Apple Music 适配器。
//!
//! 完整音频受 FairPlay DRM 保护，任何公开渠道都拿不到；本音源只提供
//! 元数据（搜索 / 专辑 / 歌单 / 榜单）与 30 秒试听。`song_url` 永远返回
//! 带 `trial` 的试听地址，由播放器的自动换源到其它平台取完整音频。
//!
//! 鉴权分两层：
//! - developer token：公开，内嵌在 music.apple.com 网页播放器的 JS 里，
//!   有效期数月。启动后首次请求时抓取，过期前自动刷新。
//! - media-user-token：用户登录 cookie，可选。有它才能取歌词（TTML）。

use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use serde_json::Value;

use crate::error::{Error, Result};
use crate::model::*;
use crate::providers::{s, u};
use crate::source::{BoxFut, Source};

const API: &str = "https://amp-api.music.apple.com/v1/catalog";
const WEB: &str = "https://music.apple.com";
/// 支持的商店区域。大陆区没有 Apple Music 目录。
pub const STOREFRONTS: &[(&str, &str)] = &[("hk", "香港"), ("tw", "台湾"), ("jp", "日本"), ("us", "美国")];
const DEFAULT_STOREFRONT: &str = "hk";
/// token 到期前多久开始刷新
const TOKEN_REFRESH_MARGIN: Duration = Duration::from_secs(24 * 3600);

#[derive(Clone)]
pub struct Apple {
    http: reqwest::Client,
    /// media-user-token（登录 cookie），仅用于歌词
    cookie: Arc<RwLock<Option<String>>>,
    storefront: Arc<RwLock<String>>,
    token: Arc<tokio::sync::RwLock<Option<DevToken>>>,
}

#[derive(Clone)]
struct DevToken {
    value: String,
    expires_at: Instant,
}

impl Default for Apple {
    fn default() -> Self {
        Self::new()
    }
}

impl Apple {
    pub fn new() -> Self {
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()
            .expect("apple http");
        Self {
            http,
            cookie: Default::default(),
            storefront: Arc::new(RwLock::new(DEFAULT_STOREFRONT.into())),
            token: Default::default(),
        }
    }

    pub fn storefront(&self) -> String {
        self.storefront.read().unwrap().clone()
    }

    /// 切换商店区域；非法值回落到默认
    pub fn set_storefront(&self, sf: &str) {
        let sf = sf.trim().to_ascii_lowercase();
        let v = if STOREFRONTS.iter().any(|(k, _)| *k == sf) { sf } else { DEFAULT_STOREFRONT.into() };
        *self.storefront.write().unwrap() = v;
    }

    // ---- developer token ----

    async fn dev_token(&self) -> Result<String> {
        if let Some(t) = self.token.read().await.as_ref() {
            if t.expires_at > Instant::now() + TOKEN_REFRESH_MARGIN {
                return Ok(t.value.clone());
            }
        }
        let mut guard = self.token.write().await;
        // 并发等待者里只有第一个真正去抓
        if let Some(t) = guard.as_ref() {
            if t.expires_at > Instant::now() + TOKEN_REFRESH_MARGIN {
                return Ok(t.value.clone());
            }
        }
        let fetched = self.fetch_dev_token().await?;
        let value = fetched.value.clone();
        *guard = Some(fetched);
        Ok(value)
    }

    async fn fetch_dev_token(&self) -> Result<DevToken> {
        let html = self.http.get(format!("{WEB}/hk/browse")).header(reqwest::header::USER_AGENT, UA).timeout(Duration::from_secs(20)).send().await?.text().await?;
        let bundle = find_between(&html, "src=\"/assets/index", ".js\"")
            .map(|mid| format!("{WEB}/assets/index{mid}.js"))
            .ok_or_else(|| Error::Parse("Apple Music 页面里找不到主脚本".into()))?;
        let js = self.http.get(&bundle).header(reqwest::header::USER_AGENT, UA).timeout(Duration::from_secs(30)).send().await?.text().await?;
        let value = extract_jwt(&js).ok_or_else(|| Error::Parse("Apple Music 脚本里找不到 developer token".into()))?;
        let exp = jwt_exp(&value).unwrap_or(0);
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        // 读不出 exp 时按 7 天处理
        let ttl = if exp > now { exp - now } else { 7 * 24 * 3600 };
        tracing::info!(ttl_days = ttl / 86400, "Apple Music developer token 已获取");
        Ok(DevToken { value, expires_at: Instant::now() + Duration::from_secs(ttl) })
    }

    // ---- HTTP ----

    async fn get(&self, path: &str, params: &[(&str, String)]) -> Result<Value> {
        let token = self.dev_token().await?;
        let sf = self.storefront();
        let url = format!("{API}/{sf}{path}");
        let mut rb = self
            .http
            .get(&url)
            .query(params)
            .bearer_auth(&token)
            .header(reqwest::header::ORIGIN, WEB)
            .header(reqwest::header::REFERER, format!("{WEB}/"))
            .header(reqwest::header::USER_AGENT, UA);
        if let Some(c) = self.cookie.read().unwrap().as_deref() {
            rb = rb.header("Media-User-Token", c);
        }
        let resp = rb.timeout(Duration::from_secs(20)).send().await?;
        let status = resp.status();
        let text = resp.text().await?;
        if status == reqwest::StatusCode::UNAUTHORIZED {
            // token 失效：清掉让下次重新抓
            *self.token.write().await = None;
            return Err(Error::Api { code: 401, msg: "Apple Music token 失效，请重试".into() });
        }
        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::Invalid("Apple Music 上没有这个内容".into()));
        }
        if !status.is_success() {
            let v: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
            let msg = s(&v["errors"][0]["detail"]);
            return Err(Error::Api { code: status.as_u16() as i64, msg: if msg.is_empty() { text.chars().take(200).collect() } else { msg } });
        }
        serde_json::from_str(&text).map_err(|_| Error::Parse("Apple Music 返回非 JSON".into()))
    }

    // ---- 数据映射 ----

    fn artwork(v: &Value, size: u32) -> String {
        let tpl = s(&v["url"]);
        if tpl.is_empty() {
            return String::new();
        }
        tpl.replace("{w}", &size.to_string()).replace("{h}", &size.to_string())
    }

    fn song(item: &Value) -> Track {
        let a = &item["attributes"];
        let artists = s(&a["artistName"]);
        let has_preview = a["previews"][0]["url"].as_str().is_some_and(|p| !p.is_empty());
        Track {
            id: s(&item["id"]),
            source: "apple".into(),
            name: s(&a["name"]),
            alias: None,
            artists: artists.split(" & ").map(|n| Artist { id: 0, name: n.trim().to_owned() }).filter(|x| !x.name.is_empty()).collect(),
            album_id: album_id_from_url(&s(&a["url"])),
            album: s(&a["albumName"]),
            cover: Self::artwork(&a["artwork"], 600),
            duration: u(&a["durationInMillis"]),
            fee: 0,
            // 完整音频永远不可得；标记为 Vip 让 UI 显示试听而不是无版权
            availability: if has_preview { Availability::Vip } else { Availability::None },
        }
    }

    fn brief(item: &Value, kind: CollectionKind) -> PlaylistBrief {
        let a = &item["attributes"];
        PlaylistBrief {
            id: s(&item["id"]),
            source: "apple".into(),
            kind,
            name: s(&a["name"]),
            cover: Self::artwork(&a["artwork"], 600),
            subtitle: {
                let c = s(&a["curatorName"]);
                if c.is_empty() { s(&a["artistName"]) } else { c }
            },
        }
    }

    async fn tracks_paged(&self, path: &str) -> Result<Vec<Track>> {
        let mut out = Vec::new();
        let mut offset = 0u32;
        loop {
            let v = self.get(path, &[("limit", "100".into()), ("offset", offset.to_string())]).await?;
            let items = v["data"].as_array().cloned().unwrap_or_default();
            let n = items.len() as u32;
            out.extend(items.iter().filter(|x| s(&x["type"]) == "songs").map(Self::song));
            if v["next"].is_null() || n == 0 || offset > 3000 {
                break;
            }
            offset += n;
        }
        Ok(out)
    }
}

const UA: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Safari/605.1.15";

impl Source for Apple {
    fn id(&self) -> &'static str {
        "apple"
    }
    fn label(&self) -> &'static str {
        "Apple Music"
    }
    fn levels(&self) -> &'static [&'static str] {
        &["standard"]
    }
    fn http(&self) -> &reqwest::Client {
        &self.http
    }

    fn parse_link(&self, text: &str) -> Option<Resolved> {
        if !text.contains("music.apple.com") {
            return None;
        }
        let url = crate::link::first_url(text)?;
        let u = reqwest::Url::parse(url).ok()?;
        let segs: Vec<&str> = u.path_segments()?.collect();
        // /{sf}/album/{slug}/{id}?i={songId} | /{sf}/playlist/{slug}/{pl.xxx} | /{sf}/song/{slug}/{id}
        let kind = segs.get(1).copied()?;
        let id = segs.last().copied().filter(|x| !x.is_empty())?.to_owned();
        let song_q = u.query_pairs().find(|(k, _)| k == "i").map(|(_, v)| v.into_owned());
        Some(match kind {
            "album" => match song_q {
                Some(sid) => Resolved::Song { source: "apple".into(), id: sid },
                None => Resolved::Album { source: "apple".into(), id },
            },
            "song" => Resolved::Song { source: "apple".into(), id },
            "playlist" => Resolved::Playlist { source: "apple".into(), id },
            _ => return None,
        })
    }

    fn search<'a>(&'a self, keywords: &'a str, limit: u32, offset: u32) -> BoxFut<'a, Vec<Track>> {
        Box::pin(async move {
            let v = self
                .get(
                    "/search",
                    &[
                        ("term", keywords.to_owned()),
                        ("types", "songs".into()),
                        ("limit", limit.clamp(1, 25).to_string()),
                        ("offset", offset.to_string()),
                    ],
                )
                .await?;
            Ok(v["results"]["songs"]["data"].as_array().map(|a| a.iter().map(Self::song).collect()).unwrap_or_default())
        })
    }

    fn track<'a>(&'a self, id: &'a str) -> BoxFut<'a, Track> {
        Box::pin(async move {
            let v = self.get(&format!("/songs/{id}"), &[]).await?;
            v["data"][0].as_object().map(|_| Self::song(&v["data"][0])).ok_or_else(|| Error::Invalid("歌曲不存在".into()))
        })
    }

    fn collection(&self, kind: CollectionKind, id: &str) -> BoxFut<'_, Collection> {
        let id = id.to_string();
        Box::pin(async move {
            let (path, tracks_path) = match kind {
                CollectionKind::Album => (format!("/albums/{id}"), format!("/albums/{id}/tracks")),
                CollectionKind::Playlist | CollectionKind::Toplist => (format!("/playlists/{id}"), format!("/playlists/{id}/tracks")),
            };
            let (meta, tracks) = tokio::join!(self.get(&path, &[]), self.tracks_paged(&tracks_path));
            let meta = meta?;
            let tracks = tracks?;
            let a = &meta["data"][0]["attributes"];
            let owner = {
                let c = s(&a["curatorName"]);
                if c.is_empty() { s(&a["artistName"]) } else { c }
            };
            let description = {
                let d = s(&a["description"]["standard"]);
                if d.is_empty() { s(&a["editorialNotes"]["standard"]) } else { d }
            };
            Ok(Collection {
                kind,
                id,
                source: "apple".into(),
                name: s(&a["name"]),
                cover: Self::artwork(&a["artwork"], 600),
                owner,
                description: strip_tags(&description),
                publish_time: parse_date_ms(&s(&a["releaseDate"])).or_else(|| parse_date_ms(&s(&a["lastModifiedDate"]))),
                track_count: tracks.len(),
                tracks,
            })
        })
    }

    fn song_url<'a>(&'a self, id: &'a str, _level: &'a str) -> BoxFut<'a, SongUrl> {
        Box::pin(async move {
            let t = self.get(&format!("/songs/{id}"), &[]).await?;
            let a = &t["data"][0]["attributes"];
            let url = s(&a["previews"][0]["url"]);
            if url.is_empty() {
                return Err(Error::Unavailable);
            }
            Ok(SongUrl {
                id: id.to_string(),
                url,
                // 试听片段只有一种规格，不随请求的音质变化
                level: "standard".into(),
                size: 0,
                br: 256,
                format: "m4a".into(),
                // Apple 试听固定约 30 秒；真实起点因歌而异，播放器只用它判断"是否试听"
                trial: Some((0, 30_000)),
            })
        })
    }

    fn lyric<'a>(&'a self, id: &'a str) -> BoxFut<'a, Lyric> {
        Box::pin(async move {
            // 没有登录 cookie 时接口返回 404/401；静默返回空，让播放器回落到换源目标的歌词
            if self.cookie.read().unwrap().is_none() {
                return Ok(Lyric::default());
            }
            match self.get(&format!("/songs/{id}/lyrics"), &[]).await {
                Ok(v) => {
                    let ttml = s(&v["data"][0]["attributes"]["ttml"]);
                    Ok(Lyric { lrc: ttml_to_lrc(&ttml), tlrc: String::new() })
                }
                Err(_) => Ok(Lyric::default()),
            }
        })
    }

    fn home(&self) -> BoxFut<'_, HomeFeed> {
        Box::pin(async move {
            let chart_params: [(&str, String); 2] = [("types", "songs,playlists,albums".into()), ("limit", "20".into())];
            let rec_params: [(&str, String); 3] = [("term", "新歌".into()), ("types", "playlists".into()), ("limit", "10".into())];
            let (charts_v, search_v) = tokio::join!(self.get("/charts", &chart_params), self.get("/search", &rec_params));
            let charts_v = charts_v?;
            let r = &charts_v["results"];
            // 榜单：Apple 的"熱門播放清單"是官方榜单
            let charts: Vec<PlaylistBrief> = r["playlists"][0]["data"]
                .as_array()
                .map(|a| a.iter().take(10).map(|x| Self::brief(x, CollectionKind::Toplist)).collect())
                .unwrap_or_default();
            // 推荐歌单：搜索结果里的编辑歌单 + 热门专辑
            let mut playlists: Vec<PlaylistBrief> = search_v
                .ok()
                .and_then(|v| v["results"]["playlists"]["data"].as_array().map(|a| a.iter().take(10).map(|x| Self::brief(x, CollectionKind::Playlist)).collect()))
                .unwrap_or_default();
            if let Some(albums) = r["albums"][0]["data"].as_array() {
                playlists.extend(albums.iter().take(6).map(|x| Self::brief(x, CollectionKind::Album)));
            }
            let new_songs: Vec<Track> = r["songs"][0]["data"].as_array().map(|a| a.iter().take(20).map(Self::song).collect()).unwrap_or_default();
            Ok(HomeFeed { playlists, charts, new_songs })
        })
    }

    fn options(&self) -> Vec<crate::source::SourceOption> {
        vec![crate::source::SourceOption {
            key: "storefront".into(),
            label: "商店区域".into(),
            value: self.storefront(),
            choices: STOREFRONTS.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
        }]
    }

    fn set_option(&self, key: &str, value: &str) {
        if key == "storefront" {
            self.set_storefront(value);
        }
    }

    fn cookie_hint(&self) -> &'static str {
        "粘贴 music.apple.com 已登录的 Cookie 中的 media-user-token（可粘整段 Cookie）。仅用于获取歌词；完整音频受 DRM 保护，播放始终自动换源到其它平台。"
    }

    fn set_cookie(&self, cookie: Option<String>) {
        // 接受裸 token 或整段 cookie 字符串
        let v = cookie.map(|c| extract_media_user_token(&c)).filter(|c| !c.is_empty());
        *self.cookie.write().unwrap() = v;
    }

    fn has_cookie(&self) -> bool {
        self.cookie.read().unwrap().is_some()
    }

    fn sized_cover(&self, url: &str, size: u32) -> String {
        // .../600x600bb.jpg → .../{size}x{size}bb.jpg
        if let Some(pos) = url.rfind('/') {
            let (base, file) = url.split_at(pos + 1);
            if let Some(rest) = file.split_once('x').and_then(|(w, rest)| w.parse::<u32>().ok().map(|_| rest)) {
                if let Some(suffix) = rest.find(|c: char| !c.is_ascii_digit()).map(|i| &rest[i..]) {
                    return format!("{base}{size}x{size}{suffix}");
                }
            }
        }
        url.to_owned()
    }

    fn is_allowed_media_url(&self, url: &str) -> bool {
        let Ok(u) = reqwest::Url::parse(url) else { return false };
        if u.scheme() != "https" {
            return false;
        }
        let host = u.host_str().unwrap_or("");
        host.ends_with(".itunes.apple.com") || host.ends_with(".mzstatic.com") || host.ends_with(".apple.com")
    }
}

// ---- 工具 ----

fn find_between<'a>(text: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let i = text.find(start)? + start.len();
    let j = text[i..].find(end)? + i;
    Some(&text[i..j])
}

/// 从网页播放器 JS 里找 kid=WebPlayKid 的 ES256 JWT
fn extract_jwt(js: &str) -> Option<String> {
    // header 固定为 {"typ":"JWT","alg":"ES256","kid":"WebPlayKid"} 的 base64url
    const HEADER: &str = "eyJ0eXAiOiJKV1QiLCJhbGciOiJFUzI1NiIsImtpZCI6IldlYlBsYXlLaWQifQ";
    let i = js.find(HEADER)?;
    let rest = &js[i..];
    let end = rest.find(|c: char| !(c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-')).unwrap_or(rest.len());
    let tok = &rest[..end];
    (tok.matches('.').count() == 2).then(|| tok.to_owned())
}

fn jwt_exp(token: &str) -> Option<u64> {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD as B64, Engine as _};
    let payload = token.split('.').nth(1)?;
    let bytes = B64.decode(payload).ok()?;
    let v: Value = serde_json::from_slice(&bytes).ok()?;
    v["exp"].as_u64()
}

/// 用户可能粘整段 cookie（`a=b; media-user-token=xxx; c=d`）或裸 token
fn extract_media_user_token(raw: &str) -> String {
    let raw = raw.trim();
    // 整段 cookie：按 ; 切分后找名字匹配的那一项
    for part in raw.split(';') {
        if let Some((k, v)) = part.trim().split_once('=') {
            if k.trim().eq_ignore_ascii_case("media-user-token") {
                return v.trim().trim_matches('"').to_owned();
            }
        }
    }
    // 裸 token 是 base64：`=` 只会出现在末尾作填充；中间有 `=` 说明是别的 cookie 项
    let body = raw.trim_end_matches('=');
    if raw.contains(' ') || raw.contains(';') || body.contains('=') || body.is_empty() {
        return String::new();
    }
    raw.to_owned()
}

/// `https://music.apple.com/hk/album/slug/1721464893?i=1721464906` → `1721464893`
fn album_id_from_url(url: &str) -> String {
    reqwest::Url::parse(url)
        .ok()
        .and_then(|u| u.path_segments().and_then(|mut p| p.next_back().map(str::to_owned)))
        .unwrap_or_default()
}

fn parse_date_ms(d: &str) -> Option<i64> {
    // "2003-07-29" 或 "2024-01-05T12:00:00Z"
    let date = d.get(..10)?;
    let mut it = date.split('-');
    let y: i64 = it.next()?.parse().ok()?;
    let m: i64 = it.next()?.parse().ok()?;
    let day: i64 = it.next()?.parse().ok()?;
    // 公历转 Unix 天数（Howard Hinnant 算法）
    let (y, m) = if m <= 2 { (y - 1, m + 9) } else { (y, m - 3) };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * m + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;
    Some(days * 86_400_000)
}

fn strip_tags(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&#39;", "'").trim().to_owned()
}

/// Apple 歌词是 TTML：`<p begin="00:12.340" end="...">歌词</p>`，内部可能有逐字 `<span>`。
/// 转成行级 LRC。
fn ttml_to_lrc(ttml: &str) -> String {
    let mut lines = Vec::new();
    let mut rest = ttml;
    while let Some(i) = rest.find("<p ") {
        let p = &rest[i..];
        let Some(open_end) = p.find('>') else { break };
        let attrs = &p[..open_end];
        let Some(close) = p.find("</p>") else { break };
        let inner = &p[open_end + 1..close];
        if let Some(begin) = find_between(attrs, "begin=\"", "\"") {
            if let Some(ms) = ttml_time_ms(begin) {
                let text = strip_tags(inner);
                let m = ms / 60_000;
                let sec = (ms % 60_000) / 1000;
                let cs = (ms % 1000) / 10;
                lines.push(format!("[{m:02}:{sec:02}.{cs:02}]{text}"));
            }
        }
        rest = &p[close + 4..];
    }
    lines.join("\n")
}

/// "12.340" | "01:12.340" | "1:01:12.340" | "12340ms" → 毫秒
fn ttml_time_ms(t: &str) -> Option<u64> {
    if let Some(ms) = t.strip_suffix("ms") {
        return ms.parse::<f64>().ok().map(|v| v as u64);
    }
    if let Some(sec) = t.strip_suffix('s') {
        return sec.parse::<f64>().ok().map(|v| (v * 1000.0) as u64);
    }
    let parts: Vec<&str> = t.split(':').collect();
    let mut total = 0.0;
    for p in &parts {
        total = total * 60.0 + p.parse::<f64>().ok()?;
    }
    Some((total * 1000.0) as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_links() {
        let a = Apple::new();
        assert_eq!(
            a.parse_link("https://music.apple.com/hk/album/%E6%99%B4%E5%A4%A9/1721464893?i=1721464906&uo=4"),
            Some(Resolved::Song { source: "apple".into(), id: "1721464906".into() })
        );
        assert_eq!(
            a.parse_link("https://music.apple.com/hk/album/葉惠美/1721464893"),
            Some(Resolved::Album { source: "apple".into(), id: "1721464893".into() })
        );
        assert_eq!(
            a.parse_link("看看这个 https://music.apple.com/hk/playlist/a-list/pl.31991c6bcf0447ac9f16c07678a5a0a0 很好听"),
            Some(Resolved::Playlist { source: "apple".into(), id: "pl.31991c6bcf0447ac9f16c07678a5a0a0".into() })
        );
        assert_eq!(a.parse_link("https://music.163.com/song?id=1"), None);
        assert_eq!(a.parse_link("晴天 周杰伦"), None);
    }

    #[test]
    fn sized_cover_rewrites_dimensions() {
        let a = Apple::new();
        let u = "https://is1-ssl.mzstatic.com/image/thumb/Music126/v4/8c/47/86/x.jpg/600x600bb.jpg";
        assert_eq!(a.sized_cover(u, 120), "https://is1-ssl.mzstatic.com/image/thumb/Music126/v4/8c/47/86/x.jpg/120x120bb.jpg");
        assert_eq!(a.sized_cover("https://other/x.jpg", 120), "https://other/x.jpg");
    }

    #[test]
    fn media_user_token_from_cookie_or_bare() {
        assert_eq!(extract_media_user_token("  AbC123==  "), "AbC123==");
        assert_eq!(extract_media_user_token("geo=HK; media-user-token=\"XyZ\"; itua=HK"), "XyZ");
        assert_eq!(extract_media_user_token("foo=bar"), "");
    }

    #[test]
    fn ttml_converts_to_lrc() {
        let ttml = r#"<tt><body><div><p begin="00:12.340" end="00:15.000" itunes:key="L1"><span begin="00:12.340">晴</span><span begin="00:12.800">天</span></p><p begin="1:01:02.5" end="1:01:05">second</p></div></body></tt>"#;
        assert_eq!(ttml_to_lrc(ttml), "[00:12.34]晴天\n[61:02.50]second");
        assert_eq!(ttml_time_ms("12.5s"), Some(12_500));
        assert_eq!(ttml_time_ms("750ms"), Some(750));
    }

    #[test]
    fn dates_and_album_ids() {
        // 2003-07-29 00:00 UTC
        assert_eq!(parse_date_ms("2003-07-29"), Some(1_059_436_800_000));
        assert_eq!(parse_date_ms("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(parse_date_ms("bad"), None);
        assert_eq!(album_id_from_url("https://music.apple.com/hk/album/%E6%99%B4%E5%A4%A9/1721464893?i=1721464906"), "1721464893");
    }

    #[test]
    fn jwt_extraction() {
        let js = r#"x=1,qa="eyJ0eXAiOiJKV1QiLCJhbGciOiJFUzI1NiIsImtpZCI6IldlYlBsYXlLaWQifQ.eyJpc3MiOiJBTVBXZWJQbGF5IiwiaWF0IjoxNzkyNzgwNTM0LCJleHAiOjE3OTg4Mjg1MzR9.sig_part-123",b=2"#;
        let tok = extract_jwt(js).unwrap();
        assert!(tok.ends_with("sig_part-123"));
        assert_eq!(jwt_exp(&tok), Some(1_798_828_534));
    }
}
