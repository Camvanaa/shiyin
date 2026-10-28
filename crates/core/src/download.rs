//! 单曲下载：获取地址、流式写盘、写入元数据、生成最终文件名。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;

use crate::api::Netease;
use crate::error::{Error, Result};
use crate::model::Track;
use crate::tag::{self, Format, Meta};
use crate::{lrc, Lyric};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DownloadOptions {
    pub level: String,
    /// 支持 ${title} ${artist} ${album} ${track}
    pub template: String,
    pub write_meta: bool,
    pub write_cover: bool,
    pub compress_cover: bool,
    pub write_artist: bool,
    pub write_album: bool,
    pub write_track: bool,
    pub write_lyrics: bool,
    pub lrc_file: bool,
}

impl Default for DownloadOptions {
    fn default() -> Self {
        Self {
            level: "exhigh".into(),
            template: "${title} - ${artist}".into(),
            write_meta: true,
            write_cover: true,
            compress_cover: true,
            write_artist: true,
            write_album: true,
            write_track: true,
            write_lyrics: true,
            lrc_file: false,
        }
    }
}

/// 下载目标：单曲本身和它在合集中的位置（用于曲目号）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadItem {
    pub track: Track,
    pub index: Option<u32>,
    pub total: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Downloaded {
    pub path: PathBuf,
    pub file_name: String,
    pub lrc_path: Option<PathBuf>,
    pub size: u64,
    pub format: &'static str,
}

#[derive(Clone, Default)]
pub struct Cancel(Arc<AtomicBool>);

impl Cancel {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Relaxed);
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
    fn check(&self) -> Result<()> {
        if self.is_cancelled() {
            Err(Error::Cancelled)
        } else {
            Ok(())
        }
    }
}

const ALLOWED_HOST_SUFFIXES: &[&str] = &[".126.net", ".music.163.com", ".163.com", ".127.net"];

/// 只允许下载网易云的媒体域名，避免被当作通用代理。
pub fn is_allowed_media_url(url: &str) -> bool {
    let Ok(u) = reqwest::Url::parse(url) else { return false };
    if !matches!(u.scheme(), "https" | "http") {
        return false;
    }
    let host = u.host_str().unwrap_or("");
    ALLOWED_HOST_SUFFIXES.iter().any(|s| host.ends_with(s) || host == &s[1..])
}

pub fn sanitize(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| if matches!(c, '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|') || c.is_control() { '_' } else { c })
        .collect();
    let s = s.trim().trim_end_matches('.').trim();
    let s: String = s.chars().take(180).collect();
    if s.is_empty() {
        "untitled".into()
    } else {
        s
    }
}

pub fn render_template(template: &str, item: &DownloadItem) -> String {
    let t = &item.track;
    let tpl = if template.trim().is_empty() { "${title} - ${artist}" } else { template };
    let track = item.index.map(|i| format!("{i:02}")).unwrap_or_default();
    sanitize(
        &tpl.replace("${title}", &t.name)
            .replace("${artist}", &t.artist_names())
            .replace("${album}", &t.album)
            .replace("${track}", &track),
    )
}

fn unique_path(dir: &Path, base: &str, ext: &str) -> PathBuf {
    let mut p = dir.join(format!("{base}.{ext}"));
    let mut n = 2;
    while p.exists() {
        p = dir.join(format!("{base} ({n}).{ext}"));
        n += 1;
    }
    p
}

/// 封面按尺寸请求，由 CDN 完成缩放，本地不解码图片。
pub fn sized_cover(url: &str, size: u32) -> String {
    if url.is_empty() {
        return String::new();
    }
    let base = url.split('?').next().unwrap_or(url);
    format!("{base}?param={size}y{size}")
}

async fn fetch_bytes(api: &Netease, url: &str) -> Option<(Vec<u8>, String)> {
    if !is_allowed_media_url(url) {
        return None;
    }
    let resp = api.http().get(url).timeout(Duration::from_secs(20)).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let mime = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("image/jpeg")
        .to_owned();
    Some((resp.bytes().await.ok()?.to_vec(), mime))
}

/// 下载单曲到 `dir`。`progress(已下载, 总大小)` 在每个数据块后调用。
pub async fn download(
    api: &Netease,
    item: &DownloadItem,
    opts: &DownloadOptions,
    dir: &Path,
    cancel: &Cancel,
    mut progress: impl FnMut(u64, u64) + Send,
) -> Result<Downloaded> {
    let t = &item.track;
    let need_lyric = (opts.write_meta && opts.write_lyrics) || opts.lrc_file;
    let need_cover = opts.write_meta && opts.write_cover && !t.cover.is_empty();
    let cover_url = if opts.compress_cover { sized_cover(&t.cover, 640) } else { t.cover.clone() };

    let (song, lyric, cover) = tokio::join!(
        api.song_url(t.id, &opts.level),
        async {
            if need_lyric {
                api.lyric(t.id).await.ok()
            } else {
                None
            }
        },
        async {
            if need_cover {
                fetch_bytes(api, &cover_url).await
            } else {
                None
            }
        }
    );
    let song = song?;
    cancel.check()?;
    if !is_allowed_media_url(&song.url) {
        return Err(Error::Invalid("音频地址不在允许的域名内".into()));
    }

    tokio::fs::create_dir_all(dir).await?;
    let base = render_template(&opts.template, item);
    let part = dir.join(format!(".{}.{}.part", base, t.id));

    let result: Result<Downloaded> = async {
        let resp = api.http().get(&song.url).timeout(Duration::from_secs(60 * 30)).send().await?;
        if !resp.status().is_success() {
            return Err(Error::Api { code: resp.status().as_u16() as i64, msg: "音频下载失败".into() });
        }
        let total = resp.content_length().unwrap_or(song.size);
        let mut file = tokio::fs::File::create(&part).await?;
        let mut stream = resp.bytes_stream();
        let mut done = 0u64;
        while let Some(chunk) = stream.next().await {
            cancel.check()?;
            let chunk = chunk?;
            file.write_all(&chunk).await?;
            done += chunk.len() as u64;
            progress(done, total);
        }
        file.flush().await?;
        drop(file);
        cancel.check()?;

        let format = Format::sniff_file(&part)?;
        let merged = lyric.as_ref().map(|l: &Lyric| lrc::merge(&l.lrc, &l.tlrc)).filter(|s| !s.is_empty());

        if opts.write_meta {
            let meta = Meta {
                title: t.name.clone(),
                artist: opts.write_artist.then(|| t.artist_names()),
                album: opts.write_album.then(|| t.album.clone()),
                track: if opts.write_track { item.index.map(|i| (i, item.total)) } else { None },
                lyrics: if opts.write_lyrics { merged.clone() } else { None },
                cover,
            };
            let p = part.clone();
            tokio::task::spawn_blocking(move || tag::write(&p, format, &meta))
                .await
                .map_err(|e| Error::Tag(e.to_string()))??;
        }

        let path = unique_path(dir, &base, format.ext());
        tokio::fs::rename(&part, &path).await?;
        let file_name = path.file_name().unwrap().to_string_lossy().into_owned();

        let mut lrc_path = None;
        if opts.lrc_file {
            if let Some(l) = &merged {
                let p = path.with_extension("lrc");
                tokio::fs::write(&p, l).await?;
                lrc_path = Some(p);
            }
        }
        let size = tokio::fs::metadata(&path).await?.len();
        Ok(Downloaded { path, file_name, lrc_path, size, format: format.ext() })
    }
    .await;

    if result.is_err() {
        let _ = tokio::fs::remove_file(&part).await;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn media_whitelist() {
        assert!(is_allowed_media_url("https://m701.music.126.net/a/b.flac"));
        assert!(is_allowed_media_url("https://p2.music.126.net/x.jpg"));
        assert!(!is_allowed_media_url("https://evil.com/x.jpg"));
        assert!(!is_allowed_media_url("https://126.net.evil.com/x"));
        assert!(!is_allowed_media_url("file:///etc/passwd"));
    }

    #[test]
    fn sanitizes() {
        assert_eq!(sanitize(" a/b:c?. "), "a_b_c_");
        assert_eq!(sanitize("..."), "untitled");
    }
}
