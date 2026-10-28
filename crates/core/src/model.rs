use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artist {
    pub id: u64,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub id: u64,
    pub name: String,
    /// 别名或译名，例如 "Sunny Day"
    pub alias: Option<String>,
    pub artists: Vec<Artist>,
    pub album_id: u64,
    pub album: String,
    pub cover: String,
    pub duration: u64,
    /// 1 = VIP 歌曲，4 = 付费专辑，8 = 免费（低音质）
    pub fee: i64,
}

impl Track {
    pub fn artist_names(&self) -> String {
        self.artists.iter().map(|a| a.name.as_str()).collect::<Vec<_>>().join("/")
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CollectionKind {
    Playlist,
    Album,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Collection {
    pub kind: CollectionKind,
    pub id: u64,
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
    pub id: u64,
    pub url: String,
    pub level: String,
    pub size: u64,
    pub br: u64,
    /// 文件类型，例如 "mp3"、"flac"
    pub format: String,
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
    /// 渲染好的 SVG
    pub svg: String,
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
#[serde(tag = "type", content = "value", rename_all = "lowercase")]
pub enum Resolved {
    Song(u64),
    Playlist(u64),
    Album(u64),
    Query(String),
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
