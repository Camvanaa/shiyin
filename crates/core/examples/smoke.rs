//! 手动冒烟测试：直连各音源跑一遍主要路径。需要网络，不属于自动化测试。
//! 运行：cargo run -p shiyin-core --example smoke

use shiyin_core::download::{self, Cancel, DownloadItem, DownloadOptions};
use shiyin_core::Netease;

#[tokio::main]
async fn main() -> shiyin_core::Result<()> {
    let api = Netease::new();
    let r = api.search("晴天 周杰伦", 3, 0).await?;
    println!("search: {:?}", r.iter().map(|t| (&t.name, t.artist_names(), t.fee)).collect::<Vec<_>>());

    let p = api.playlist(3778678).await?;
    println!("playlist: {} {} tracks", p.name, p.tracks.len());

    let a = api.album(18905).await?;
    println!("album: {} by {} {} tracks", a.name, a.owner, a.tracks.len());

    let l = api.lyric(186016).await?;
    println!("lyric: {} bytes, tlrc {}", l.lrc.len(), l.tlrc.len());

    let q = api.qr_start().await?;
    println!("qr: key {} svg {} bytes", q.key.len(), q.svg.len());

    // 找一首免费歌曲测试播放地址与下载
    let free = p.tracks.iter().find(|t| t.fee == 0 || t.fee == 8).cloned();
    let Some(free) = free else {
        println!("歌单里没有免费歌曲，跳过下载测试");
        return Ok(());
    };
    let id: u64 = free.id.parse().map_err(|_| shiyin_core::Error::Invalid(free.id.clone()))?;
    match api.song_url(id, "exhigh").await {
        Ok(u) => println!("url: {} {} {}", u.level, u.format, u.size),
        Err(e) => println!("url err: {e}"),
    }

    let dir = std::path::Path::new("/tmp/opencode/dl");
    std::fs::create_dir_all(dir)?;
    let item = DownloadItem { track: free, index: Some(1), total: Some(1) };
    let opts = DownloadOptions { lrc_file: true, ..Default::default() };
    let d = download::download(&api, &item, &opts, dir, &Cancel::default(), |_, _| {}).await?;
    println!("downloaded: {d:?}");
    Ok(())
}
