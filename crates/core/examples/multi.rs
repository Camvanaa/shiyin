use shiyin_core::{CollectionKind, Sources};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let src = Sources::new(shiyin_core::Netease::new());
    for id in ["kuwo", "qq"] {
        let s = src.get(id).unwrap();
        let r = s.search("晴天 周杰伦", 2, 0).await?;
        println!("[{}] {:?}", id, r.iter().map(|t| (t.id.as_str(), t.name.as_str(), t.artist_names(), &t.cover[..28.min(t.cover.len())])).collect::<Vec<_>>());
        let t = &r[0];
        match s.song_url(&t.id, "exhigh").await {
            Ok(u) => println!("[{}] url: {} {} bytes={}", id, u.format, &u.url[..40.min(u.url.len())], u.size),
            Err(e) => println!("[{}] url 不可用：{e}", id),
        }
        match s.lyric(&t.id).await {
            Ok(l) => println!("[{}] lyric {} bytes: {:?}", id, l.lrc.len(), l.lrc.lines().next()),
            Err(e) => println!("[{}] lyric 失败：{e}", id),
        }
        match s.track(&t.id).await {
            Ok(t) => println!("[{}] track ok: {}", id, t.name),
            Err(e) => println!("[{}] track 失败：{e}", id),
        }
    }
    // 酷我歌单
    let kw = src.get("kuwo").unwrap();
    match kw.collection(CollectionKind::Playlist, "790274661").await {
        Ok(pl) => println!("[kuwo] playlist {} - {} tracks", pl.name, pl.tracks.len()),
        Err(e) => println!("[kuwo] playlist 失败：{e}"),
    }
    Ok(())
}
