//! 酷我音乐适配器。公开接口、无加密，参考 ML-Music_url/providers/kuwo.py。

use serde_json::{json, Value};

use crate::error::{Error, Result};
use crate::model::*;
use crate::providers::{extract_id, first_str, s, u};
use crate::source::{BoxFut, Source};

#[derive(Clone)]
pub struct Kuwo {
    http: reqwest::Client,
    cookie: std::sync::Arc<std::sync::RwLock<Option<String>>>,
    home_cache: std::sync::Arc<std::sync::RwLock<Option<(std::time::Instant, HomeFeed)>>>,
}

impl Default for Kuwo {
    fn default() -> Self {
        Self::new()
    }
}

fn bitrate(level: &str) -> Option<&'static str> {
    Some(match level {
        "standard" => "128kmp3",
        "exhigh" => "320kmp3",
        "lossless" => "flac",
        "hires" => "2000kflac",
        _ => return None,
    })
}

impl Kuwo {
    pub fn new() -> Self {
        let http = reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(10))
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()
            .expect("kuwo http");
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
                .header(reqwest::header::REFERER, "https://www.kuwo.cn/")
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
                .map_err(|_| Error::Parse("酷我返回非 JSON".into()))?;
            Ok(v)
        })
    }

    fn picture(v: &Value) -> String {
        let p = s(v);
        if p.is_empty() {
            return String::new();
        }
        if let Some(rest) = p.strip_prefix("//") {
            return format!("https:{rest}");
        }
        if p.starts_with("http") {
            return p.replacen("http://", "https://", 1);
        }
        if p.starts_with("img") {
            return format!("https://{p}");
        }
        format!("https://img1.kuwo.cn/star/albumcover/{}", p.trim_start_matches('/'))
    }

    fn song(item: &Value) -> Track {
        let rid = s(first_str(item, &["MUSICRID", "musicrid", "rid", "id"]));
        let id = rid.trim_start_matches("MUSIC_").to_string();
        let artists = s(first_str(item, &["ARTIST", "artist"]));
        Track {
            id: id.clone(),
            source: "kuwo".into(),
            name: s(first_str(item, &["SONGNAME", "NAME", "name", "songName"])),
            alias: None,
            artists: vec![Artist { id: 0, name: artists }],
            album_id: s(first_str(item, &["ALBUMID", "albumid", "album_id"])),
            album: s(first_str(item, &["ALBUM", "album"])),
            cover: Self::picture(first_str(item, &["web_albumpic_short", "hts_MVPIC", "hts_MUSIC_PIC_PATH", "pic", "albumpic"])),
            duration: u(first_str(item, &["DURATION", "duration"])) * 1000,
            fee: 0,
            availability: Availability::Ok,
        }
    }

    async fn detail(&self, id: &str) -> Result<(Track, Lyric)> {
        let v = self
            .get(
                "https://m.kuwo.cn/newh5/singles/songinfoandlrc",
                &[("musicId", id.to_string()), ("httpsStatus", "1".into())],
            )
            .await?;
        let data = if v["data"].is_object() { &v["data"] } else { &v };
        let info = &data["songinfo"];
        if !info.is_object() {
            return Err(Error::Invalid("酷我未返回歌曲详情".into()));
        }
        let mut t = Self::song(&json!({
            "rid": info["rid"].clone().as_str().map(str::to_owned).unwrap_or_else(|| id.to_string()),
            "songName": info["songName"].clone(),
            "artist": info["artist"].clone(),
            "album": info["album"].clone(),
            "pic": info["pic"].clone(),
            "duration": info["duration"].clone(),
        }));
        t.album_id = s(first_str(info, &["albumid", "albumId"]));
        let lyric = data["lrclist"]
            .as_array()
            .map(|lines| {
                lines
                    .iter()
                    .map(|l| {
                        let text = s(first_str(l, &["lineLyric", "line"]));
                        let t = first_str(l, &["time"]).as_f64().unwrap_or(0.0);
                        let cs = (t * 100.0).round().max(0.0) as u64;
                        let m = cs / 6000;
                        let sec = (cs % 6000) / 100;
                        let frac = cs % 100;
                        format!("[{m:02}:{sec:02}.{frac:02}]{text}")
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_default();
        Ok((t, Lyric { lrc: lyric, tlrc: String::new() }))
    }
}

impl Source for Kuwo {
    fn id(&self) -> &'static str {
        "kuwo"
    }
    fn label(&self) -> &'static str {
        "酷我音乐"
    }
    fn levels(&self) -> &'static [&'static str] {
        &["standard", "exhigh", "lossless", "hires"]
    }
    fn http(&self) -> &reqwest::Client {
        &self.http
    }

    fn parse_link(&self, text: &str) -> Option<Resolved> {
        if !text.contains("kuwo.cn") && !text.contains("MUSIC_") && !text.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        if text.contains("play_detail") || text.contains("MUSIC_") || (text.chars().all(|c| c.is_ascii_digit()) && text.len() <= 12) {
            let id = extract_id(text, &["rid", "musicId", "id"], "/play_detail/")?;
            return Some(Resolved::Song { source: "kuwo".into(), id: id.trim_start_matches("MUSIC_").into() });
        }
        if text.contains("playlist") {
            let id = extract_id(text, &["pid", "id"], "/playlist_detail/")?;
            return Some(Resolved::Playlist { source: "kuwo".into(), id });
        }
        if text.contains("album") {
            let id = extract_id(text, &["albumid", "id"], "/album/")?;
            return Some(Resolved::Album { source: "kuwo".into(), id });
        }
        None
    }

    fn search<'a>(&'a self, keywords: &'a str, limit: u32, offset: u32) -> BoxFut<'a, Vec<Track>> {
        Box::pin(async move {
            let pn = offset / limit.max(1);
            let v = self
                .get(
                    "https://www.kuwo.cn/search/searchMusicBykeyWord",
                    &[
                        ("vipver", "1".into()),
                        ("client", "kt".into()),
                        ("ft", "music".into()),
                        ("cluster", "0".into()),
                        ("strategy", "2012".into()),
                        ("encoding", "utf8".into()),
                        ("rformat", "json".into()),
                        ("mobi", "1".into()),
                        ("issubtitle", "1".into()),
                        ("show_copyright_off", "1".into()),
                        ("pn", pn.to_string()),
                        ("rn", limit.clamp(1, 100).to_string()),
                        ("all", keywords.to_string()),
                        ("httpsStatus", "1".into()),
                    ],
                )
                .await?;
            let items = if v["abslist"].is_array() {
                v["abslist"].as_array().cloned().unwrap_or_default()
            } else {
                v["data"]["list"].as_array().cloned().unwrap_or_default()
            };
            Ok(items.iter().map(Self::song).collect())
        })
    }

    fn track<'a>(&'a self, id: &'a str) -> BoxFut<'a, Track> {
        Box::pin(async move {
        self.detail(id).await.map(|(t, _)| t)
        })
    }

    fn lyric<'a>(&'a self, id: &'a str) -> BoxFut<'a, Lyric> {
        Box::pin(async move {
            match self.detail(id).await {
                Ok((_, l)) => Ok(l),
                Err(_) => Ok(Lyric::default()),
            }
        })
    }

    fn song_url<'a>(&'a self, id: &'a str, level: &'a str) -> BoxFut<'a, SongUrl> {
        Box::pin(async move {
            let br = bitrate(level).ok_or_else(|| Error::Invalid(format!("酷我不支持音质 {level}")))?;
            let v = self
                .get(
                    "https://mobi.kuwo.cn/mobi.s",
                    &[
                        ("f", "web".into()),
                        ("source", "kwplayercar_ar_6.0.0.9_B_jiakong_vh.apk".into()),
                        ("from", "PC".into()),
                        ("type", "convert_url_with_sign".into()),
                        ("br", br.to_string()),
                        ("rid", id.to_string()),
                    ],
                )
                .await?;
            let data = if v["data"].is_object() { &v["data"] } else { &v };
            let url = s(&data["url"]);
            if url.is_empty() {
                return Err(Error::Unavailable);
            }
            // 个别地区会被风控重定向到无关试听，校验 rid 一致性
            let rid = u(&data["rid"]);
            if rid != 0 && rid.to_string() != id {
                return Err(Error::Unavailable);
            }
            let format = s(&data["format"]).to_ascii_lowercase();
            Ok(SongUrl {
                id: id.to_string(),
                url: url.replacen("http://", "https://", 1),
                level: level.to_string(),
                size: u(&data["size"]),
                br: 0,
                format,
                trial: None,
            })
        })
    }

    fn collection(&self, kind: CollectionKind, id: &str) -> BoxFut<'_, Collection> {
        let id = id.to_string();
        Box::pin(async move {
            let (url, params) = match kind {
                CollectionKind::Playlist | CollectionKind::Toplist => (
                    "https://nplserver.kuwo.cn/pl.svc",
                    vec![
                        ("op", "getlistinfo".into()),
                        ("pid", id.to_string()),
                        ("encode", "utf8".into()),
                        ("keyset", "pl2012".into()),
                        ("identity", "kuwo".into()),
                        ("pcmp4", "1".into()),
                        ("vipver", "1".into()),
                        ("newver", "1".into()),
                    ],
                ),
                CollectionKind::Album => (
                    "https://search.kuwo.cn/r.s",
                    vec![("stype", "albuminfo".into()), ("albumid", id.to_string())],
                ),
            };
            // 分页拉取，每页 100
            let mut page = 0;
            let page_size = 100;
            let mut first: Option<Value> = None;
            let mut tracks: Vec<Track> = Vec::new();
            let mut owner = String::new();
            loop {
                let mut p = params.clone();
                p.push(("pn", page.to_string()));
                p.push(("rn", page_size.to_string()));
                p.push(("httpsStatus", "1".into()));
                let v = self.get(url, &p).await?;
                if v["result"].as_str() == Some("fail") || v["reason"].as_str() == Some("PARAM_ERROR") {
                    return Err(Error::Invalid("酷我集合 ID 无效".into()));
                }
                let info = if v["data"].is_object() { &v["data"] } else { &v };
                if page == 0 {
                    first = Some(info.clone());
                }
                let items = info["musiclist"].as_array().or_else(|| info["songs"].as_array()).cloned().unwrap_or_default();
                if items.is_empty() {
                    break;
                }
                if owner.is_empty() {
                    owner = s(first_str(info, &["uname", "owner", "username", "artist", "artistname"]));
                }
                let total = u(first_str(info, &["total", "songnum"]));
                tracks.extend(items.iter().map(Self::song));
                page += 1;
                if tracks.len() as u64 >= total || (items.len() as u64) < page_size || page > 50 {
                    break;
                }
            }
            let info = first.unwrap_or(Value::Null);
            Ok(Collection {
                kind,
                id: id.to_string(),
                source: "kuwo".into(),
                name: s(first_str(&info, &["title", "name", "album"])),
                cover: Self::picture(first_str(&info, &["pic", "cover", "album_pic"])),
                owner,
                description: s(first_str(&info, &["info", "intro"])),
                publish_time: None,
                track_count: tracks.len(),
                tracks,
            })
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
            let (rcm, news) = tokio::join!(
                self.get(
                    "https://wapi.kuwo.cn/api/pc/classify/playlist/getRcmPlayList",
                    &[("pn", "1".into()), ("rn", "10".into()), ("order", "hot".into()), ("httpsStatus", "1".into())],
                ),
                self.search("热歌", 12, 0),
            );
            let playlists = rcm
                .ok()
                .and_then(|v| {
                    v["data"]["data"].as_array().map(|a| {
                        a.iter()
                            .take(10)
                            .map(|x| PlaylistBrief {
                                id: s(&x["id"]),
                                source: "kuwo".into(),
                                kind: CollectionKind::Playlist,
                                name: s(&x["name"]),
                                cover: Self::picture(&x["img"]),
                                subtitle: s(&x["uname"]),
                            })
                            .collect()
                    })
                })
                .unwrap_or_default();
            let feed = HomeFeed {
                playlists,
                charts: Vec::new(),
                new_songs: news.unwrap_or_default(),
            };
            if !feed.playlists.is_empty() {
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
        host.ends_with(".kuwo.cn") || host == "kuwo.cn"
    }
}
