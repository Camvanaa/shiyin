use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Resolved {
    Song { source: String, id: String },
    Playlist { source: String, id: String },
    Album { source: String, id: String },
    Query(String),
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
