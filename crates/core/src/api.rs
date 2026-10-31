use std::sync::{Arc, RwLock};
use std::time::Duration;

use rand::Rng;
use reqwest::header::{HeaderMap, HeaderValue, COOKIE, REFERER, SET_COOKIE, USER_AGENT};
use serde_json::{json, Value};

use crate::crypto::eapi_params;
use crate::error::{Error, Result};
use crate::model::*;

const UA_PC: &str = "Mozilla/5.0 (Windows NT 10.0; WOW64) AppleWebKit/537.36 (KHTML, like Gecko) Safari/537.36 Chrome/91.0.4472.164 NeteaseMusicDesktop/2.10.2.200154";
const OS: &str = "pc";
const APPVER: &str = "8.9.75";
const DEVICE_ID: &str = "pyncm!";

/// 网易云接口客户端。克隆开销很小，可在任务间共享。
#[derive(Clone)]
pub struct Netease {
    http: reqwest::Client,
    music_u: Arc<RwLock<Option<String>>>,
    /// 部署在海外时设置一个国内 IP，网易云据此判断版权地区
    real_ip: Option<HeaderValue>,
    home_cache: Arc<RwLock<Option<(std::time::Instant, HomeFeed)>>>,
}

fn s(v: &Value) -> String {
    v.as_str().unwrap_or_default().to_owned()
}

fn u(v: &Value) -> u64 {
    v.as_u64().or_else(|| v.as_f64().map(|f| f as u64)).unwrap_or(0)
}

fn check_code(v: &Value) -> Result<()> {
    match v.get("code").and_then(Value::as_i64) {
        None | Some(200) => Ok(()),
        Some(code) => Err(Error::Api {
            code,
            msg: v
                .get("message")
                .or_else(|| v.get("msg"))
                .and_then(Value::as_str)
                .unwrap_or("未知错误")
                .to_owned(),
        }),
    }
}

fn availability(fee: i64, p: &Value) -> Availability {
    if p.is_null() {
        return Availability::Ok;
    }
    let st = p["st"].as_i64().unwrap_or(0);
    let pl = p["pl"].as_i64().unwrap_or(0);
    if st < 0 {
        return Availability::None;
    }
    if pl > 0 {
        return Availability::Ok;
    }
    match fee {
        4 => Availability::Paid,
        _ => Availability::Vip,
    }
}

pub(crate) fn parse_track(v: &Value) -> Track {
    parse_track_with(v, &v["privilege"])
}

pub(crate) fn parse_track_with(v: &Value, privilege: &Value) -> Track {
    let fee = v["fee"].as_i64().unwrap_or(0);
    let al = &v["al"];
    let alias = v["tns"][0]
        .as_str()
        .or_else(|| v["alia"][0].as_str())
        .filter(|a| !a.is_empty())
        .map(str::to_owned);
    Track {
        id: u(&v["id"]).to_string(),
        source: "netease".into(),
        name: s(&v["name"]),
        alias,
        artists: v["ar"]
            .as_array()
            .map(|a| a.iter().map(|x| Artist { id: u(&x["id"]), name: s(&x["name"]) }).collect())
            .unwrap_or_default(),
        album_id: u(&al["id"]).to_string(),
        album: s(&al["name"]),
        cover: s(&al["picUrl"]).replacen("http://", "https://", 1),
        duration: u(&v["dt"]),
        fee,
        availability: availability(fee, privilege),
    }
}

impl Default for Netease {
    fn default() -> Self {
        Self::new()
    }
}

impl Netease {
    pub fn new() -> Self {
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .pool_idle_timeout(Duration::from_secs(60))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("http client");
        Self {
            http,
            music_u: Arc::new(RwLock::new(None)),
            real_ip: None,
            home_cache: Arc::new(RwLock::new(None)),
        }
    }

    pub fn with_real_ip(mut self, ip: Option<std::net::IpAddr>) -> Self {
        self.real_ip = ip.and_then(|ip| HeaderValue::from_str(&ip.to_string()).ok());
        self
    }

    pub fn http(&self) -> &reqwest::Client {
        &self.http
    }

    pub fn set_music_u(&self, value: Option<String>) {
        let v = value.map(|s| s.trim().trim_start_matches("MUSIC_U=").trim_end_matches(';').to_owned());
        *self.music_u.write().unwrap() = v.filter(|s| !s.is_empty());
    }

    pub fn music_u(&self) -> Option<String> {
        self.music_u.read().unwrap().clone()
    }

    fn cookie_header(&self) -> HeaderValue {
        let mut c = format!("os={OS}; appver={APPVER}; deviceId={DEVICE_ID}");
        if let Some(mu) = self.music_u() {
            c.push_str("; MUSIC_U=");
            c.push_str(&mu);
        }
        HeaderValue::from_str(&c).unwrap_or_else(|_| HeaderValue::from_static(""))
    }

    fn weapi_headers(&self) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(USER_AGENT, HeaderValue::from_static("Mozilla/5.0"));
        h.insert(REFERER, HeaderValue::from_static("https://music.163.com/"));
        h.insert(COOKIE, self.cookie_header());
        if let Some(ip) = &self.real_ip {
            h.insert("X-Real-IP", ip.clone());
        }
        h
    }

    async fn api_form(&self, url: &str, form: &[(&str, String)]) -> Result<Value> {
        let resp = self
            .http
            .post(url)
            .headers(self.weapi_headers())
            .form(form)
            .timeout(Duration::from_secs(20))
            .send()
            .await?;
        let v: Value = resp.json().await?;
        check_code(&v)?;
        Ok(v)
    }

    async fn eapi(&self, path: &str, mut payload: Value) -> Result<reqwest::Response> {
        let header = json!({
            "os": OS,
            "appver": APPVER,
            "osver": "",
            "deviceId": DEVICE_ID,
            "requestId": rand::thread_rng().gen_range(20_000_000..30_000_000).to_string(),
        });
        payload["header"] = Value::String(header.to_string());
        let api_path = path.replacen("/eapi/", "/api/", 1);
        let params = eapi_params(&api_path, &payload.to_string());
        let mut rb = self
            .http
            .post(format!("https://interface3.music.163.com{path}"))
            .header(USER_AGENT, UA_PC)
            .header(COOKIE, self.cookie_header());
        if let Some(ip) = &self.real_ip {
            rb = rb.header("X-Real-IP", ip.clone());
        }
        let resp = rb
            .form(&[("params", params)])
            .timeout(Duration::from_secs(20))
            .send()
            .await?;
        Ok(resp)
    }

    pub async fn search(&self, keywords: &str, limit: u32, offset: u32) -> Result<Vec<Track>> {
        let v = self
            .api_form(
                "https://music.163.com/api/cloudsearch/pc",
                &[
                    ("s", keywords.to_owned()),
                    ("type", "1".into()),
                    ("limit", limit.clamp(1, 100).to_string()),
                    ("offset", offset.to_string()),
                ],
            )
            .await?;
        Ok(v["result"]["songs"].as_array().map(|a| a.iter().map(parse_track).collect()).unwrap_or_default())
    }

    pub async fn song_details(&self, ids: &[u64]) -> Result<Vec<Track>> {
        let mut out = Vec::with_capacity(ids.len());
        // 分批并发请求，缩短大歌单的加载时间。
        let batches = ids.chunks(500).map(|chunk| {
            let c = Value::Array(chunk.iter().map(|id| json!({ "id": id, "v": 0 })).collect()).to_string();
            async move { self.api_form("https://interface3.music.163.com/api/v3/song/detail", &[("c", c)]).await }
        });
        for v in futures_util::future::join_all(batches).await {
            let v = v?;
            if let Some(a) = v["songs"].as_array() {
                // privileges 与 songs 顺序一致，但按 id 匹配更稳妥
                let privs: std::collections::HashMap<u64, &Value> = v["privileges"]
                    .as_array()
                    .map(|p| p.iter().map(|x| (u(&x["id"]), x)).collect())
                    .unwrap_or_default();
                out.extend(a.iter().map(|t| parse_track_with(t, privs.get(&u(&t["id"])).copied().unwrap_or(&Value::Null))));
            }
        }
        Ok(out)
    }

    pub async fn song_url(&self, id: u64, level: &str) -> Result<SongUrl> {
        if !is_valid_level(level) {
            return Err(Error::Invalid(format!("未知音质：{level}")));
        }
        let mut payload = json!({ "ids": [id], "level": level, "encodeType": "flac" });
        if level == "sky" {
            payload["immerseType"] = "c51".into();
        }
        let v: Value = self.eapi("/eapi/song/enhance/player/url/v1", payload).await?.json().await?;
        check_code(&v)?;
        let d = &v["data"][0];
        let url = d["url"].as_str().ok_or(Error::Unavailable)?;
        Ok(SongUrl {
            id: u(&d["id"]).to_string(),
            url: url.replacen("http://", "https://", 1),
            level: s(&d["level"]),
            size: u(&d["size"]),
            br: u(&d["br"]),
            format: s(&d["type"]).to_ascii_lowercase(),
            trial: d["freeTrialInfo"]
                .as_object()
                .map(|t| (u(&t["start"]) , u(&t["end"])))
                .map(|(a, b)| if b < 10_000 { (a * 1000, b * 1000) } else { (a, b) }),
        })
    }

    pub async fn lyric(&self, id: u64) -> Result<Lyric> {
        let v = self
            .api_form(
                "https://interface3.music.163.com/api/song/lyric",
                &[("id", id.to_string()), ("lv", "-1".into()), ("tv", "-1".into()), ("kv", "-1".into())],
            )
            .await?;
        Ok(Lyric { lrc: s(&v["lrc"]["lyric"]), tlrc: s(&v["tlyric"]["lyric"]) })
    }

    pub async fn playlist(&self, id: u64) -> Result<Collection> {
        let v = self
            .api_form("https://music.163.com/api/v6/playlist/detail", &[("id", id.to_string()), ("n", "0".into())])
            .await?;
        let p = &v["playlist"];
        if p.is_null() {
            return Err(Error::Invalid("歌单不存在".into()));
        }
        let ids: Vec<u64> = p["trackIds"].as_array().map(|a| a.iter().map(|t| u(&t["id"])).collect()).unwrap_or_default();
        let tracks = self.song_details(&ids).await?;
        Ok(Collection {
            kind: CollectionKind::Playlist,
            id: u(&p["id"]).to_string(),
            source: "netease".into(),
            name: s(&p["name"]),
            cover: s(&p["coverImgUrl"]).replacen("http://", "https://", 1),
            owner: s(&p["creator"]["nickname"]),
            description: s(&p["description"]),
            publish_time: p["createTime"].as_i64(),
            track_count: tracks.len(),
            tracks,
        })
    }

    pub async fn album(&self, id: u64) -> Result<Collection> {
        let resp = self
            .http
            .get(format!("https://music.163.com/api/v1/album/{id}"))
            .headers(self.weapi_headers())
            .timeout(Duration::from_secs(20))
            .send()
            .await?;
        let v: Value = resp.json().await?;
        check_code(&v)?;
        let a = &v["album"];
        if a.is_null() {
            return Err(Error::Invalid("专辑不存在".into()));
        }
        let mut tracks: Vec<Track> = v["songs"].as_array().map(|x| x.iter().map(parse_track).collect()).unwrap_or_default();
        let cover = s(&a["picUrl"]).replacen("http://", "https://", 1);
        for t in &mut tracks {
            if t.cover.is_empty() {
                t.cover = cover.clone();
            }
        }
        Ok(Collection {
            kind: CollectionKind::Album,
            id: u(&a["id"]).to_string(),
            source: "netease".into(),
            name: s(&a["name"]),
            cover,
            owner: a["artists"]
                .as_array()
                .map(|x| x.iter().map(|y| s(&y["name"])).collect::<Vec<_>>().join("/"))
                .filter(|x| !x.is_empty())
                .unwrap_or_else(|| s(&a["artist"]["name"])),
            description: s(&a["description"]),
            publish_time: a["publishTime"].as_i64(),
            track_count: tracks.len(),
            tracks,
        })
    }

    pub async fn account(&self) -> Result<Option<Account>> {
        if self.music_u().is_none() {
            return Ok(None);
        }
        let v = self.api_form("https://music.163.com/api/nuser/account/get", &[]).await?;
        let p = &v["profile"];
        if p.is_null() {
            return Ok(None);
        }
        Ok(Some(Account {
            user_id: u(&p["userId"]),
            nickname: s(&p["nickname"]),
            avatar: s(&p["avatarUrl"]).replacen("http://", "https://", 1),
            vip_type: p["vipType"].as_i64().unwrap_or(0),
        }))
    }

    pub async fn qr_start(&self) -> Result<QrStart> {
        let v: Value = self.eapi("/eapi/login/qrcode/unikey", json!({ "type": 1 })).await?.json().await?;
        check_code(&v)?;
        let key = s(&v["unikey"]);
        if key.is_empty() {
            return Err(Error::Parse("未返回二维码 key".into()));
        }
        let url = format!("https://music.163.com/login?codekey={key}");
        let svg = qrcode::QrCode::new(url.as_bytes())
            .map_err(|e| Error::Parse(e.to_string()))?
            .render::<qrcode::render::svg::Color>()
            .quiet_zone(false)
            .dark_color(qrcode::render::svg::Color("currentColor"))
            .light_color(qrcode::render::svg::Color("transparent"))
            .build();
        Ok(QrStart { key, url, svg })
    }

    /// 查询扫码状态。成功时返回 MUSIC_U。
    pub async fn qr_check(&self, key: &str) -> Result<(QrState, Option<String>)> {
        let resp = self.eapi("/eapi/login/qrcode/client/login", json!({ "key": key, "type": 1 })).await?;
        let music_u = resp
            .headers()
            .get_all(SET_COOKIE)
            .iter()
            .filter_map(|h| h.to_str().ok())
            .find_map(|c| c.strip_prefix("MUSIC_U=").map(|r| r.split(';').next().unwrap_or("").to_owned()))
            .filter(|s| !s.is_empty());
        let v: Value = resp.json().await?;
        let state = match v["code"].as_i64() {
            Some(801) => QrState::Waiting,
            Some(802) => QrState::Scanned,
            Some(803) => QrState::Success,
            Some(800) => QrState::Expired,
            Some(code) => return Err(Error::Api { code, msg: s(&v["message"]) }),
            None => return Err(Error::Parse("扫码状态未知".into())),
        };
        if state == QrState::Success && music_u.is_none() {
            return Err(Error::Parse("登录成功但未获取到凭证".into()));
        }
        Ok((state, music_u))
    }

    /// 首页推荐内容：推荐歌单、官方榜单、最新音乐。内存缓存 15 分钟。
    pub async fn home(&self) -> Result<HomeFeed> {
        if let Some((t, feed)) = self.home_cache.read().unwrap().as_ref() {
            if t.elapsed() < Duration::from_secs(900) {
                return Ok(feed.clone());
            }
        }
        let limit = [("limit", String::from("12"))];
        let (pl, top, ns) = tokio::join!(
            self.api_form("https://music.163.com/api/personalized/playlist", &limit),
            self.api_form("https://music.163.com/api/toplist", &[]),
            self.api_form("https://music.163.com/api/personalized/newsong", &limit),
        );

        let playlists = pl
            .ok()
            .and_then(|v| {
                v["result"].as_array().map(|a| {
                    a.iter()
                        .take(10)
                        .map(|x| PlaylistBrief {
                            id: u(&x["id"]).to_string(),
                            source: "netease".into(),
                            kind: CollectionKind::Playlist,
                            name: s(&x["name"]),
                            cover: s(&x["picUrl"]).replacen("http://", "https://", 1),
                            subtitle: s(&x["copywriter"]),
                        })
                        .collect()
                })
            })
            .unwrap_or_default();

        let charts = top
            .ok()
            .and_then(|v| {
                v["list"].as_array().map(|a| {
                    a.iter()
                        .take(8)
                        .map(|x| PlaylistBrief {
                            id: u(&x["id"]).to_string(),
                            source: "netease".into(),
                            kind: CollectionKind::Playlist,
                            name: s(&x["name"]),
                            cover: s(&x["coverImgUrl"]).replacen("http://", "https://", 1),
                            subtitle: s(&x["updateFrequency"]),
                        })
                        .collect()
                })
            })
            .unwrap_or_default();

        let new_songs = ns
            .ok()
            .and_then(|v| {
                v["result"].as_array().map(|a| {
                    a.iter()
                        .filter_map(|x| {
                            let sg = &x["song"];
                            if sg.is_null() {
                                return None;
                            }
                            let fee = sg["fee"].as_i64().unwrap_or(0);
                            let al = &sg["album"];
                            Some(Track {
                                id: u(&sg["id"]).to_string(),
                                source: "netease".into(),
                                name: s(&sg["name"]),
                                alias: sg["alias"][0].as_str().filter(|a| !a.is_empty()).map(str::to_owned),
                                artists: sg["artists"]
                                    .as_array()
                                    .map(|ar| ar.iter().map(|y| Artist { id: u(&y["id"]), name: s(&y["name"]) }).collect())
                                    .unwrap_or_default(),
                                album_id: u(&al["id"]).to_string(),
                                album: s(&al["name"]),
                                cover: x["picUrl"]
                                    .as_str()
                                    .filter(|c| !c.is_empty())
                                    .map(str::to_owned)
                                    .unwrap_or_else(|| s(&al["picUrl"]))
                                    .replacen("http://", "https://", 1),
                                duration: u(&sg["duration"]),
                                fee,
                                availability: availability(fee, &sg["privilege"]),
                            })
                        })
                        .collect()
                })
            })
            .unwrap_or_default();

        let feed = HomeFeed { playlists, charts, new_songs };
        if !feed.playlists.is_empty() || !feed.charts.is_empty() {
            *self.home_cache.write().unwrap() = Some((std::time::Instant::now(), feed.clone()));
        }
        Ok(feed)
    }
}
