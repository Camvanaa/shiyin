//! 写入音频元数据。按文件头识别格式，不依赖音质推断。

use std::fs::File;
use std::io::Read;
use std::path::Path;

use crate::error::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Mp3,
    Flac,
}

impl Format {
    pub fn ext(self) -> &'static str {
        match self {
            Format::Mp3 => "mp3",
            Format::Flac => "flac",
        }
    }

    pub fn mime(self) -> &'static str {
        match self {
            Format::Mp3 => "audio/mpeg",
            Format::Flac => "audio/flac",
        }
    }

    pub fn sniff(head: &[u8]) -> Format {
        if head.starts_with(b"fLaC") {
            return Format::Flac;
        }
        // ID3 头后可能紧跟 FLAC（少见），按 ID3 长度跳过再判断。
        if head.len() >= 10 && head.starts_with(b"ID3") {
            let size = ((head[6] as usize & 0x7f) << 21)
                | ((head[7] as usize & 0x7f) << 14)
                | ((head[8] as usize & 0x7f) << 7)
                | (head[9] as usize & 0x7f);
            if head.get(10 + size..10 + size + 4) == Some(b"fLaC") {
                return Format::Flac;
            }
        }
        Format::Mp3
    }

    pub fn sniff_file(path: &Path) -> Result<Format> {
        let mut head = vec![0u8; 4096];
        let n = File::open(path)?.read(&mut head)?;
        Ok(Format::sniff(&head[..n]))
    }
}

#[derive(Debug, Clone, Default)]
pub struct Meta {
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub track: Option<(u32, Option<u32>)>,
    pub lyrics: Option<String>,
    /// (数据, MIME)
    pub cover: Option<(Vec<u8>, String)>,
}

fn cover_mime(data: &[u8], hint: &str) -> String {
    if data.starts_with(&[0x89, b'P', b'N', b'G']) {
        "image/png".into()
    } else if data.starts_with(&[0xff, 0xd8]) {
        "image/jpeg".into()
    } else if hint.starts_with("image/") {
        hint.replace("image/jpg", "image/jpeg")
    } else {
        "image/jpeg".into()
    }
}

pub fn write(path: &Path, format: Format, meta: &Meta) -> Result<()> {
    match format {
        Format::Mp3 => write_id3(path, meta),
        Format::Flac => write_flac(path, meta),
    }
}

fn write_id3(path: &Path, m: &Meta) -> Result<()> {
    use id3::frame::{Lyrics, Picture, PictureType};
    use id3::{Tag, TagLike, Version};

    let mut tag = Tag::new();
    tag.set_title(&m.title);
    if let Some(a) = &m.artist {
        tag.set_artist(a);
    }
    if let Some(a) = &m.album {
        tag.set_album(a);
    }
    if let Some((n, total)) = m.track {
        tag.set_track(n);
        if let Some(t) = total {
            tag.set_total_tracks(t);
        }
    }
    if let Some(l) = &m.lyrics {
        tag.add_frame(Lyrics { lang: "und".into(), description: String::new(), text: l.clone() });
    }
    if let Some((data, hint)) = &m.cover {
        tag.add_frame(Picture {
            mime_type: cover_mime(data, hint),
            picture_type: PictureType::CoverFront,
            description: "Cover".into(),
            data: data.clone(),
        });
    }
    tag.write_to_path(path, Version::Id3v23).map_err(|e| Error::Tag(e.to_string()))
}

fn write_flac(path: &Path, m: &Meta) -> Result<()> {
    use metaflac::block::PictureType;
    use metaflac::Tag;

    let mut tag = Tag::read_from_path(path).map_err(|e| Error::Tag(e.to_string()))?;
    let vc = tag.vorbis_comments_mut();
    vc.set_title(vec![m.title.clone()]);
    if let Some(a) = &m.artist {
        vc.set_artist(vec![a.clone()]);
    }
    if let Some(a) = &m.album {
        vc.set_album(vec![a.clone()]);
    }
    if let Some((n, total)) = m.track {
        vc.set_track(n);
        if let Some(t) = total {
            vc.set_total_tracks(t);
        }
    }
    if let Some(l) = &m.lyrics {
        vc.set_lyrics(vec![l.clone()]);
    }
    if let Some((data, hint)) = &m.cover {
        tag.remove_picture_type(PictureType::CoverFront);
        tag.add_picture(cover_mime(data, hint), PictureType::CoverFront, data.clone());
    }
    tag.save().map_err(|e| Error::Tag(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sniffs() {
        assert_eq!(Format::sniff(b"fLaC\0\0\0"), Format::Flac);
        assert_eq!(Format::sniff(b"ID3\x04\0\0\0\0\0\0"), Format::Mp3);
        assert_eq!(Format::sniff(&[0xff, 0xfb, 0x90]), Format::Mp3);
    }
}
