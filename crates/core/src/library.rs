//! 资料库：喜欢的歌曲、自建歌单、播放记录。
//!
//! 持久化为数据目录下的单个 JSON 文件（原子写：先写 .tmp 再 rename），
//! 启动时读入内存，写操作立即落盘。选型说明见 docs/DESIGN-v3.md 3.11：
//! 个人库数据量小（数千条），JSON 足以胜任，且避免为 Android/Windows
//! 交叉编译引入 SQLite 的 bundled C 依赖。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::model::{Artist, Track};

/// 资料库里保存的曲目快照（不依赖原平台仍可展示）。
/// 与 `Track` 的差别：不含易变的 availability，只留展示与再解析所需字段。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LibTrack {
    pub id: String,
    pub source: String,
    pub name: String,
    #[serde(default)]
    pub artists: Vec<Artist>,
    #[serde(default)]
    pub album: String,
    #[serde(default)]
    pub album_id: String,
    #[serde(default)]
    pub cover: String,
    #[serde(default)]
    pub duration: u64,
    /// 合并键（有则用于跨平台再解析）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub isrc: Option<String>,
    /// 其它平台的同名版本（"source:id"），用于播放时择优
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub alts: Vec<String>,
    /// 加入时间（Unix 毫秒）
    pub added_at: i64,
}

impl LibTrack {
    /// 由当前曲目 + 已知的其它版本构建。
    pub fn from_track(t: &Track, alts: Vec<String>, now: i64) -> Self {
        Self {
            id: t.id.clone(),
            source: t.source.clone(),
            name: t.name.clone(),
            artists: t.artists.clone(),
            album: t.album.clone(),
            album_id: t.album_id.clone(),
            cover: t.cover.clone(),
            duration: t.duration,
            isrc: t.isrc.clone().filter(|s| !s.trim().is_empty()),
            alts,
            added_at: now,
        }
    }

    /// 唯一键：`source:id`。
    pub fn key(&self) -> String {
        format!("{}:{}", self.source, self.id)
    }

    /// 还原为可播放的 `Track`（availability 交给解析器判定，这里给 Ok）。
    pub fn to_track(&self) -> Track {
        Track {
            id: self.id.clone(),
            source: self.source.clone(),
            name: self.name.clone(),
            alias: None,
            artists: self.artists.clone(),
            album_id: self.album_id.clone(),
            album: self.album.clone(),
            cover: self.cover.clone(),
            duration: self.duration,
            fee: 0,
            availability: crate::model::Availability::Ok,
            isrc: self.isrc.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Playlist {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub cover: String,
    pub created_at: i64,
    pub updated_at: i64,
    #[serde(default)]
    pub tracks: Vec<LibTrack>,
}

/// 播放记录的一条：曲目 + 最近播放时间 + 次数。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlayRecord {
    #[serde(flatten)]
    pub track: LibTrack,
    pub played_at: i64,
    #[serde(default)]
    pub count: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Data {
    #[serde(default)]
    likes: Vec<LibTrack>,
    #[serde(default)]
    playlists: Vec<Playlist>,
    #[serde(default)]
    plays: Vec<PlayRecord>,
}

/// 资料库。内存态 + 落盘。
pub struct Library {
    path: PathBuf,
    data: Data,
}

/// 播放记录上限，防止无限增长
const MAX_PLAYS: usize = 500;

impl Library {
    pub fn new(dir: &Path) -> Self {
        let path = dir.join("library.json");
        let data = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str::<Data>(&s).ok())
            .unwrap_or_default();
        Self { path, data }
    }

    /// 原子写入：先写临时文件再 rename，避免中断导致文件损坏。
    fn persist(&self) -> Result<()> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = self.path.with_extension("json.tmp");
        let json = serde_json::to_vec(&self.data)?;
        std::fs::write(&tmp, &json)?;
        std::fs::rename(&tmp, &self.path)?;
        Ok(())
    }

    pub fn likes(&self) -> &[LibTrack] {
        &self.data.likes
    }

    pub fn is_liked(&self, source: &str, id: &str) -> bool {
        let k = format!("{source}:{id}");
        self.data.likes.iter().any(|t| t.key() == k)
    }

    /// 切换喜欢状态，返回切换后是否已喜欢。
    pub fn toggle_like(&mut self, t: &Track, alts: Vec<String>, now: i64) -> Result<bool> {
        let k = format!("{}:{}", t.source, t.id);
        if let Some(pos) = self.data.likes.iter().position(|x| x.key() == k) {
            self.data.likes.remove(pos);
            self.persist()?;
            return Ok(false);
        }
        self.data.likes.insert(0, LibTrack::from_track(t, alts, now));
        self.persist()?;
        Ok(true)
    }

    pub fn playlists(&self) -> &[Playlist] {
        &self.data.playlists
    }

    pub fn playlist(&self, id: &str) -> Option<&Playlist> {
        self.data.playlists.iter().find(|p| p.id == id)
    }

    pub fn create_playlist(&mut self, name: &str, now: i64) -> Result<Playlist> {
        let id = format!("pl-{now}-{}", self.data.playlists.len());
        let pl = Playlist {
            id: id.clone(),
            name: name.trim().to_owned(),
            description: String::new(),
            cover: String::new(),
            created_at: now,
            updated_at: now,
            tracks: Vec::new(),
        };
        self.data.playlists.insert(0, pl.clone());
        self.persist()?;
        Ok(pl)
    }

    pub fn rename_playlist(&mut self, id: &str, name: &str, now: i64) -> Result<bool> {
        let Some(pl) = self.data.playlists.iter_mut().find(|p| p.id == id) else {
            return Ok(false);
        };
        pl.name = name.trim().to_owned();
        pl.updated_at = now;
        self.persist()?;
        Ok(true)
    }

    pub fn delete_playlist(&mut self, id: &str) -> Result<bool> {
        let before = self.data.playlists.len();
        self.data.playlists.retain(|p| p.id != id);
        let removed = self.data.playlists.len() != before;
        if removed {
            self.persist()?;
        }
        Ok(removed)
    }

    /// 向歌单添加曲目，返回新加入的数量（已在其中的跳过）。
    pub fn add_to_playlist(&mut self, id: &str, tracks: Vec<(Track, Vec<String>)>, now: i64) -> Result<usize> {
        let Some(pl) = self.data.playlists.iter_mut().find(|p| p.id == id) else {
            return Ok(0);
        };
        let existing: std::collections::HashSet<String> = pl.tracks.iter().map(LibTrack::key).collect();
        let mut added = 0;
        for (t, alts) in tracks {
            let item = LibTrack::from_track(&t, alts, now);
            if existing.contains(&item.key()) {
                continue;
            }
            pl.tracks.push(item);
            added += 1;
        }
        if added > 0 {
            pl.updated_at = now;
            if pl.cover.is_empty() {
                pl.cover = pl.tracks.first().map(|t| t.cover.clone()).unwrap_or_default();
            }
            self.persist()?;
        }
        Ok(added)
    }

    /// 从歌单移除曲目（按 `source:id`）。
    pub fn remove_from_playlist(&mut self, id: &str, track_key: &str) -> Result<bool> {
        let Some(pl) = self.data.playlists.iter_mut().find(|p| p.id == id) else {
            return Ok(false);
        };
        let before = pl.tracks.len();
        pl.tracks.retain(|t| t.key() != track_key);
        let removed = pl.tracks.len() != before;
        if removed {
            self.persist()?;
        }
        Ok(removed)
    }

    pub fn plays(&self) -> &[PlayRecord] {
        &self.data.plays
    }

    /// 记录一次播放：已在记录里的移到最前并计数 +1，否则插入。
    pub fn record_play(&mut self, t: &Track, alts: Vec<String>, now: i64) -> Result<()> {
        let k = format!("{}:{}", t.source, t.id);
        if let Some(pos) = self.data.plays.iter().position(|r| r.track.key() == k) {
            let mut rec = self.data.plays.remove(pos);
            rec.played_at = now;
            rec.count += 1;
            self.data.plays.insert(0, rec);
        } else {
            self.data.plays.insert(
                0,
                PlayRecord { track: LibTrack::from_track(t, alts, now), played_at: now, count: 1 },
            );
        }
        self.data.plays.truncate(MAX_PLAYS);
        self.persist()
    }

    pub fn clear_plays(&mut self) -> Result<()> {
        self.data.plays.clear();
        self.persist()
    }

    /// 导入时的去重键（供未来导入他站歌单使用）。
    pub fn like_keys(&self) -> BTreeMap<String, i64> {
        self.data.likes.iter().map(|t| (t.key(), t.added_at)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Availability;

    fn tmpdir() -> PathBuf {
        let d = std::env::temp_dir().join(format!("shiyin-lib-test-{}", rand::random::<u32>()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn track(id: &str, source: &str, name: &str) -> Track {
        Track {
            id: id.into(),
            source: source.into(),
            name: name.into(),
            alias: None,
            artists: vec![Artist { id: 0, name: "周杰伦".into() }],
            album_id: "a".into(),
            album: "叶子".into(),
            cover: "c.jpg".into(),
            duration: 269000,
            fee: 0,
            availability: Availability::Ok,
            isrc: Some("TWK1".into()),
        }
    }

    #[test]
    fn like_toggle_and_persist() {
        let dir = tmpdir();
        let mut lib = Library::new(&dir);
        assert!(!lib.is_liked("qq", "1"));
        assert!(lib.toggle_like(&track("1", "qq", "晴天"), vec![], 100).unwrap());
        assert!(lib.is_liked("qq", "1"));
        // 重新打开应读回
        let lib2 = Library::new(&dir);
        assert!(lib2.is_liked("qq", "1"));
        assert_eq!(lib2.likes()[0].name, "晴天");
        assert_eq!(lib2.likes()[0].isrc.as_deref(), Some("TWK1"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn like_toggle_removes() {
        let dir = tmpdir();
        let mut lib = Library::new(&dir);
        lib.toggle_like(&track("1", "qq", "晴天"), vec![], 100).unwrap();
        assert!(!lib.toggle_like(&track("1", "qq", "晴天"), vec![], 200).unwrap());
        assert!(lib.likes().is_empty());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn playlist_crud_and_dedup() {
        let dir = tmpdir();
        let mut lib = Library::new(&dir);
        let pl = lib.create_playlist(" 我的最爱 ", 100).unwrap();
        assert_eq!(pl.name, "我的最爱");
        let a = track("1", "qq", "晴天");
        let b = track("2", "kuwo", "夜曲");
        assert_eq!(lib.add_to_playlist(&pl.id, vec![(a.clone(), vec![]), (b.clone(), vec![])], 100).unwrap(), 2);
        // 重复添加被跳过
        assert_eq!(lib.add_to_playlist(&pl.id, vec![(a.clone(), vec![])], 100).unwrap(), 0);
        assert_eq!(lib.playlist(&pl.id).unwrap().tracks.len(), 2);
        // 首曲封面自动成为歌单封面
        assert_eq!(lib.playlist(&pl.id).unwrap().cover, "c.jpg");
        // 移除
        assert!(lib.remove_from_playlist(&pl.id, "qq:1").unwrap());
        assert_eq!(lib.playlist(&pl.id).unwrap().tracks.len(), 1);
        assert!(lib.rename_playlist(&pl.id, "新的名字", 200).unwrap());
        assert_eq!(lib.playlist(&pl.id).unwrap().name, "新的名字");
        assert!(lib.delete_playlist(&pl.id).unwrap());
        assert!(lib.playlist(&pl.id).is_none());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn add_to_missing_playlist_is_noop() {
        let dir = tmpdir();
        let mut lib = Library::new(&dir);
        assert_eq!(lib.add_to_playlist("nope", vec![(track("1", "qq", "a"), vec![])], 1).unwrap(), 0);
        assert!(!lib.remove_from_playlist("nope", "qq:1").unwrap());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn play_records_dedupe_and_reorder() {
        let dir = tmpdir();
        let mut lib = Library::new(&dir);
        lib.record_play(&track("1", "qq", "晴天"), vec![], 100).unwrap();
        lib.record_play(&track("2", "kuwo", "夜曲"), vec![], 200).unwrap();
        lib.record_play(&track("1", "qq", "晴天"), vec![], 300).unwrap();
        let plays = lib.plays();
        assert_eq!(plays.len(), 2);
        // 最近播放的排最前，且计数为 2
        assert_eq!(plays[0].track.name, "晴天");
        assert_eq!(plays[0].count, 2);
        assert_eq!(plays[1].track.name, "夜曲");
        lib.clear_plays().unwrap();
        assert!(lib.plays().is_empty());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn corrupt_file_falls_back_to_empty() {
        let dir = tmpdir();
        std::fs::write(dir.join("library.json"), b"{ not json").unwrap();
        let lib = Library::new(&dir);
        assert!(lib.likes().is_empty());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn lib_track_roundtrips_to_track() {
        let t = track("1", "qq", "晴天");
        let lt = LibTrack::from_track(&t, vec!["kuwo:2".into()], 5);
        let back = lt.to_track();
        assert_eq!(back.id, "1");
        assert_eq!(back.source, "qq");
        assert_eq!(back.artists.len(), 1);
        assert_eq!(lt.alts, vec!["kuwo:2".to_string()]);
    }
}
