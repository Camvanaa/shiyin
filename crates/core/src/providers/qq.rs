//! QQ 音乐适配器。参考 ML-Music_url/providers/qq.py，播放走 GET CgiGetVkey（免签名）。

use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use serde_json::{json, Value};

use crate::error::{Error, Result};
use crate::model::*;
use crate::providers::{extract_id, first_str, s, u};
use crate::source::{BoxFut, Source};

#[derive(Clone)]
pub struct QQ {
    http: reqwest::Client,
    cookie: std::sync::Arc<std::sync::RwLock<Option<String>>>,
    home_cache: std::sync::Arc<std::sync::RwLock<Option<(std::time::Instant, HomeFeed)>>>,
}

impl Default for QQ {
    fn default() -> Self {
        Self::new()
    }
}

fn quality(level: &str) -> &'static [(&'static str, &'static str)] {
    // (文件前缀, 扩展名)，按优先级降级
    match level {
        "standard" => &[("M500", "mp3")],
        "exhigh" => &[("M800", "mp3"), ("M500", "mp3")],
        _ => &[("F000", "flac"), ("M800", "mp3"), ("M500", "mp3")], // lossless / hires
    }
}

impl QQ {
    pub fn new() -> Self {
        let http = reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("qq http");
        Self {
            http,
            cookie: Default::default(),
            home_cache: Default::default(),
        }
    }

    fn get(&self, url: &str, params: &[(&str, String)]) -> BoxFut<'static, Value> {
        let url = url.to_owned();
        let params: Vec<(String, String)> = params.iter().map(|(k, v)| (k.to_string(), v.clone())).collect();
        let this = self.clone();
        Box::pin(async move {
            let mut rb = this
                .http
                .get(&url)
                .query(&params)
                .header(reqwest::header::REFERER, "https://y.qq.com/")
                .header(reqwest::header::USER_AGENT, "Mozilla/5.0");
            if let Some(c) = this.cookie.read().unwrap().as_deref() {
                rb = rb.header(reqwest::header::COOKIE, c);
            }
            let resp = rb
                .timeout(std::time::Duration::from_secs(20))
                .send()
                .await?;
            let text = resp.text().await?;
            let v: Value = serde_json::from_str(text.trim().trim_start_matches("callback(").trim_end_matches([')', ';']))
                .map_err(|_| Error::Parse("QQ 返回非 JSON".into()))?;
            Ok(v)
        })
    }

    fn cover(album_mid: &str) -> String {
        if album_mid.is_empty() {
            String::new()
        } else {
            format!("https://y.gtimg.cn/music/photo_new/T002R300x300M000{album_mid}.jpg")
        }
    }

    fn song(item: &Value) -> Track {
        let item = if item["songInfo"].is_object() { &item["songInfo"] } else { item };
        let mid = s(first_str(item, &["songmid", "mid"]));
        let numeric = s(first_str(item, &["songid", "id"]));
        let album = &item["album"];
        let album_mid = {
            let m = s(first_str(item, &["albummid"]));
            if m.is_empty() { s(&album["mid"]) } else { m }
        };
        let singers = if item["singer"].is_array() { &item["singer"] } else { &item["singers"] };
        let artists: Vec<Artist> = singers
            .as_array()
            .map(|a| a.iter().map(|x| Artist { id: 0, name: s(first_str(x, &["name", "singer_name"])) }).collect())
            .unwrap_or_else(|| vec![Artist { id: 0, name: s(first_str(item, &["singername", "singerName"])) }]);
        Track {
            id: if mid.is_empty() { numeric } else { mid },
            source: "qq".into(),
            name: s(first_str(item, &["songname", "name", "title"])),
            alias: None,
            artists,
            album_id: album_mid.clone(),
            album: {
                let n = s(first_str(item, &["albumname"]));
                if n.is_empty() { s(&album["name"]) } else { n }
            },
            cover: Self::cover(&album_mid),
            duration: u(first_str(item, &["interval", "duration"])) * 1000,
            fee: 0,
            availability: Availability::Ok,
        }
    }

    fn cookie_val(&self, keys: &[&str]) -> String {
        let Some(raw) = self.cookie.read().unwrap().clone() else { return String::new() };
        for part in raw.split(';') {
            if let Some((k, v)) = part.trim().split_once('=') {
                if keys.contains(&k.trim()) {
                    return v.trim().to_owned();
                }
            }
        }
        String::new()
    }

    async fn musicu(&self, body: Value) -> Result<Value> {
        let mut rb = self
            .http
            .post("https://u.y.qq.com/cgi-bin/musicu.fcg")
            .header(reqwest::header::REFERER, "https://y.qq.com/")
            .header(reqwest::header::USER_AGENT, "Mozilla/5.0")
            .json(&body)
            .timeout(std::time::Duration::from_secs(20));
        if let Some(c) = self.cookie.read().unwrap().as_deref() {
            rb = rb.header(reqwest::header::COOKIE, c);
        }
        Ok(rb.send().await?.json().await?)
    }

    async fn vkey(&self, mid: &str, prefix: &str, ext: &str) -> Result<(String, i64)> {
        let filename = format!("{prefix}{mid}.{ext}");
        let uin = {
            let u = self.cookie_val(&["uin", "wxuin"]);
            let u = u.trim_start_matches('o').to_owned();
            if u.is_empty() { "0".into() } else { u }
        };
        let authst = self.cookie_val(&["qqmusic_key", "qm_keyst"]);
        let payload = json!({
            "req_0": {
                "module": "vkey.GetVkeyServer",
                "method": "CgiGetVkey",
                "param": {
                    "filename": [filename],
                    "guid": format!("{:032x}", rand::random::<u128>()),
                    "songmid": [mid],
                    "songtype": [0],
                    "uin": uin,
                    "loginflag": 1,
                    "platform": "20",
                },
            },
            "comm": {
                "uin": uin.parse::<u64>().unwrap_or(0),
                "format": "json",
                "ct": 19,
                "cv": 0,
                "authst": authst,
            },
        });
        let v = self
            .get(
                "https://u.y.qq.com/cgi-bin/musicu.fcg",
                &[
                    ("-", "getplaysongvkey".into()),
                    ("g_tk", "5381".into()),
                    ("loginUin", "0".into()),
                    ("hostUin", "0".into()),
                    ("format", "json".into()),
                    ("inCharset", "utf8".into()),
                    ("outCharset", "utf-8".into()),
                    ("notice", "0".into()),
                    ("platform", "yqq.json".into()),
                    ("needNewCode", "0".into()),
                    ("data", payload.to_string()),
                ],
            )
            .await?;
        let data = &v["req_0"]["data"];
        let retcode = data["retcode"].as_i64().unwrap_or(0);
        let purl = data["midurlinfo"][0]["purl"].as_str().unwrap_or("").to_string();
        Ok((purl, retcode))
    }
}

impl Source for QQ {
    fn id(&self) -> &'static str {
        "qq"
    }
    fn label(&self) -> &'static str {
        "QQ音乐"
    }
    fn levels(&self) -> &'static [&'static str] {
        &["standard", "exhigh", "lossless", "hires"]
    }
    fn http(&self) -> &reqwest::Client {
        &self.http
    }

    fn parse_link(&self, text: &str) -> Option<Resolved> {
        if !text.contains("qq.com") {
            return None;
        }
        if text.contains("songDetail") || text.contains("songmid") || text.contains("songid") {
            let id = extract_id(text, &["songmid", "songid", "mid", "id"], "/songDetail/")?;
            return Some(Resolved::Song { source: "qq".into(), id });
        }
        if text.contains("playlist") || text.contains("taoge") || text.contains("disstid") {
            let id = extract_id(text, &["disstid", "id"], "/playlist/")?;
            return Some(Resolved::Playlist { source: "qq".into(), id });
        }
        if text.contains("albumDetail") || text.contains("albummid") {
            let id = extract_id(text, &["albummid", "albumid", "id"], "/albumDetail/")?;
            return Some(Resolved::Album { source: "qq".into(), id });
        }
        None
    }

    fn search<'a>(&'a self, keywords: &'a str, limit: u32, offset: u32) -> BoxFut<'a, Vec<Track>> {
        Box::pin(async move {
            let n = limit.clamp(1, 100);
            let page = offset / n + 1;
            let v = self
                .get(
                    "https://c.y.qq.com/soso/fcgi-bin/search_for_qq_cp",
                    &[
                        ("w", keywords.to_string()),
                        ("format", "json".into()),
                        ("p", page.to_string()),
                        ("n", n.to_string()),
                    ],
                )
                .await?;
            let items = v["data"]["song"]["list"].as_array().cloned().unwrap_or_default();
            Ok(items.iter().map(Self::song).collect())
        })
    }

    fn track<'a>(&'a self, id: &'a str) -> BoxFut<'a, Track> {
        Box::pin(async move {
            let v = self
                .get(
                    "https://c.y.qq.com/v8/fcg-bin/fcg_play_single_song.fcg",
                    &[("songmid", id.to_string()), ("format", "json".into()), ("platform", "yqq".into())],
                )
                .await?;
            let item = v["data"].as_array().and_then(|a| a.first()).cloned().ok_or_else(|| Error::Invalid("QQ 未返回歌曲详情".into()))?;
            Ok(Self::song(&item))
        })
    }

    fn song_url<'a>(&'a self, id: &'a str, level: &'a str) -> BoxFut<'a, SongUrl> {
        Box::pin(async move {
            let mut last = 0i64;
            for (prefix, ext) in quality(level) {
                let (purl, retcode) = self.vkey(id, prefix, ext).await?;
                last = retcode;
                if purl.is_empty() {
                    continue;
                }
                let url = if purl.starts_with("http") {
                    purl
                } else {
                    format!("https://dl.stream.qqmusic.qq.com/{purl}")
                };
                return Ok(SongUrl {
                    id: id.to_string(),
                    url: url.replacen("http://", "https://", 1),
                    level: level.to_string(),
                    size: 0,
                    br: 0,
                    format: ext.to_string(),
                    trial: None,
                });
            }
            if last == 104003 {
                Err(Error::Invalid("QQ：当前歌曲或音质需要绿钻或登录（104003）".into()))
            } else {
                Err(Error::Unavailable)
            }
        })
    }

    fn lyric<'a>(&'a self, id: &'a str) -> BoxFut<'a, Lyric> {
        Box::pin(async move {
            let v = self
                .get(
                    "https://c.y.qq.com/lyric/fcgi-bin/fcg_query_lyric_new.fcg",
                    &[("songmid", id.to_string()), ("format", "json".into()), ("nobase64", "1".into())],
                )
                .await?;
            let mut lrc = s(&v["lyric"]);
            let tlrc = s(&v["trans"]);
            // 若仍返回 base64，则解码
            if !lrc.contains('\n') && !lrc.contains('[') && !lrc.is_empty() {
                if let Ok(bytes) = B64.decode(&lrc) {
                    if let Ok(text) = String::from_utf8(bytes) {
                        lrc = text;
                    }
                }
            }
            Ok(Lyric { lrc, tlrc })
        })
    }

    fn collection(&self, kind: CollectionKind, id: &str) -> BoxFut<'_, Collection> {
        let id = id.to_string();
        Box::pin(async move {
            match kind {
                CollectionKind::Toplist => {
                    let top_id: u64 = id.parse().map_err(|_| Error::Invalid("QQ 榜单 ID 无效".into()))?;
                    let v = self
                        .musicu(json!({
                            "comm": { "ct": 24, "cv": 0 },
                            "req_0": {
                                "module": "musicToplist.ToplistInfoServer",
                                "method": "GetDetail",
                                "param": { "topId": top_id, "offset": 0, "num": 100 },
                            }
                        }))
                        .await?;
                    let data = &v["req_0"]["data"];
                    let items = data["songInfoList"].as_array().cloned().unwrap_or_default();
                    let tracks: Vec<Track> = items.iter().map(Self::song).collect();
                    let info = &data["data"];
                    Ok(Collection {
                        kind: CollectionKind::Toplist,
                        id: id.to_string(),
                        source: "qq".into(),
                        name: s(&info["title"]),
                        cover: s(first_str(info, &["frontPicUrl", "headPicUrl"])).replacen("http://", "https://", 1),
                        owner: "QQ音乐".into(),
                        description: s(&info["intro"]),
                        publish_time: None,
                        track_count: tracks.len(),
                        tracks,
                    })
                }
                CollectionKind::Playlist => {
                    let v = self
                        .get(
                            "https://i.y.qq.com/qzone-music/fcg-bin/fcg_ucc_getcdinfo_byids_cp.fcg",
                            &[
                                ("type", "1".into()),
                                ("json", "1".into()),
                                ("utf8", "1".into()),
                                ("onlysong", "0".into()),
                                ("disstid", id.to_string()),
                                ("format", "json".into()),
                            ],
                        )
                        .await?;
                    let info = v["cdlist"].as_array().and_then(|a| a.first()).cloned().ok_or_else(|| Error::Invalid("QQ 未返回歌单".into()))?;
                    let items = info["songlist"].as_array().cloned().unwrap_or_default();
                    let tracks: Vec<Track> = items.iter().map(Self::song).collect();
                    Ok(Collection {
                        kind,
                        id: id.to_string(),
                        source: "qq".into(),
                        name: s(&info["dissname"]),
                        cover: s(&info["logo"]).replacen("http://", "https://", 1),
                        owner: s(&info["nickname"]),
                        description: s(&info["desc"]),
                        publish_time: None,
                        track_count: tracks.len(),
                        tracks,
                    })
                }
                CollectionKind::Album => {
                    let key = if id.chars().all(|c| c.is_ascii_digit()) { "albumID" } else { "albumMid" };
                    let param_val = if key == "albumID" { json!(id.parse::<u64>().unwrap_or(0)) } else { json!(id) };
                    let post = |method: &str, param: Value| {
                        let module = if method == "GetAlbumDetail" {
                            "music.musichallAlbum.AlbumInfoServer"
                        } else {
                            "music.musichallAlbum.AlbumSongList"
                        };
                        let body = json!({ "comm": { "ct": 24, "cv": 0 }, "req_0": { "module": module, "method": method, "param": param } });
                        async move {
                            let resp = self
                                .http
                                .post("https://u.y.qq.com/cgi-bin/musicu.fcg")
                                .header(reqwest::header::REFERER, "https://y.qq.com/")
                                .json(&body)
                                .timeout(std::time::Duration::from_secs(20))
                                .send()
                                .await?;
                            let v: Value = resp.json().await?;
                            Ok::<Value, Error>(v["req_0"]["data"].clone())
                        }
                    };
                    let info = post("GetAlbumDetail", json!({ key: param_val })).await?;
                    let mut tracks = Vec::new();
                    let mut begin = 0;
                    let page_size = 100;
                    let mut total = 0u64;
                    for _ in 0..50 {
                        let r = post("GetAlbumSongList", json!({ key: param_val, "begin": begin, "num": page_size, "order": 2 })).await?;
                        let items = r["songList"].as_array().or_else(|| r["songlist"].as_array()).cloned().unwrap_or_default();
                        if items.is_empty() {
                            break;
                        }
                        total = u(first_str(&r, &["totalNum", "totalSongNum", "total"]));
                        tracks.extend(items.iter().map(Self::song));
                        begin += items.len();
                        if tracks.len() as u64 >= total || items.len() < page_size {
                            break;
                        }
                    }
                    let basic = if info["basicInfo"].is_object() { &info["basicInfo"] } else { &info };
                    let album_mid = {
                        let m = s(first_str(basic, &["albumMid", "albummid"]));
                        if m.is_empty() && key == "albumMid" { id.to_string() } else { m }
                    };
                    let artist = {
                        let list = info["singer"]["singerList"].as_array().cloned().unwrap_or_default();
                        let names: Vec<String> = list.iter().map(|x| s(&x["name"])).filter(|n| !n.is_empty()).collect();
                        if names.is_empty() { s(first_str(basic, &["singerName", "singername"])) } else { names.join("/") }
                    };
                    Ok(Collection {
                        kind,
                        id: if album_mid.is_empty() { id.to_string() } else { album_mid.clone() },
                        source: "qq".into(),
                        name: s(first_str(basic, &["albumName", "albumname", "name"])),
                        cover: Self::cover(&album_mid),
                        owner: artist,
                        description: s(first_str(basic, &["desc", "description"])),
                        publish_time: None,
                        track_count: if total > 0 { total as usize } else { tracks.len() },
                        tracks,
                    })
                }
            }
        })
    }

    fn set_cookie(&self, cookie: Option<String>) {
        *self.cookie.write().unwrap() = cookie.map(|s| s.trim().to_owned()).filter(|s| !s.is_empty());
    }

    fn has_cookie(&self) -> bool {
        self.cookie.read().unwrap().is_some()
    }

    fn home(&self) -> BoxFut<'_, HomeFeed> {
        Box::pin(async move {
            if let Some((t, f)) = self.home_cache.read().unwrap().as_ref() {
                if t.elapsed() < std::time::Duration::from_secs(900) {
                    return Ok(f.clone());
                }
            }
            let v = self
                .musicu(json!({
                    "comm": { "ct": 24, "cv": 0 },
                    "recom": {
                        "module": "playlist.HotRecommendServer",
                        "method": "get_hot_recommend",
                        "param": { "cmd": 0, "async": 1 }
                    },
                    "top": {
                        "module": "musicToplist.ToplistInfoServer",
                        "method": "GetAll",
                        "param": {}
                    },
                    "new": {
                        "module": "newsong.NewSongServer",
                        "method": "get_new_song_info",
                        "param": { "type": 5 }
                    }
                }))
                .await?;
            let playlists = v["recom"]["data"]["v_hot"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .take(10)
                        .map(|x| PlaylistBrief {
                            id: s(&x["content_id"]),
                            source: "qq".into(),
                            kind: CollectionKind::Playlist,
                            name: s(&x["title"]),
                            cover: s(&x["cover"]).replacen("http://", "https://", 1),
                            subtitle: s(&x["rcmdcontent"]),
                        })
                        .collect()
                })
                .unwrap_or_default();
            let mut charts = Vec::new();
            if let Some(groups) = v["top"]["data"]["group"].as_array() {
                for g in groups {
                    if let Some(list) = g["toplist"].as_array() {
                        for t in list {
                            if charts.len() >= 8 {
                                break;
                            }
                            charts.push(PlaylistBrief {
                                id: s(&t["topId"]),
                                source: "qq".into(),
                                kind: CollectionKind::Toplist,
                                name: s(&t["title"]),
                                cover: s(&t["frontPicUrl"]).replacen("http://", "https://", 1),
                                subtitle: s(&t["updateTips"]),
                            });
                        }
                    }
                }
            }
            let new_songs = v["new"]["data"]["lanlist"]
                .as_array()
                .and_then(|a| a.first())
                .and_then(|x| x["songlist"].as_array())
                .or_else(|| v["new"]["data"]["songlist"].as_array())
                .map(|a| a.iter().take(12).map(Self::song).collect())
                .unwrap_or_default();
            let feed = HomeFeed { playlists, charts, new_songs };
            if !feed.playlists.is_empty() || !feed.charts.is_empty() {
                *self.home_cache.write().unwrap() = Some((std::time::Instant::now(), feed.clone()));
            }
            Ok(feed)
        })
    }

    fn is_allowed_media_url(&self, url: &str) -> bool {
        let Ok(u) = reqwest::Url::parse(url) else { return false };
        if !matches!(u.scheme(), "https" | "http") {
            return false;
        }
        let host = u.host_str().unwrap_or("");
        host.ends_with(".qq.com") || host.ends_with(".gtimg.cn") || host.ends_with(".qqmusic.qq.com")
    }
}
