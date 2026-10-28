use shiyin_core::{download::*, *};

#[tokio::main]
async fn main() -> Result<()> {
    let api = Netease::new();
    let r = api.search("晴天 周杰伦", 3, 0).await?;
    println!("search: {:?}", r.iter().map(|t| (&t.name, t.artist_names(), t.fee)).collect::<Vec<_>>());
    let p = api.playlist(3778678).await?;
    println!("playlist: {} {} tracks", p.name, p.tracks.len());
    let a = api.album(18905).await?;
    println!("album: {} by {} {} tracks", a.name, a.owner, a.tracks.len());
    let l = api.lyric(186016).await?;
    println!("lyric: {} bytes, tlrc {}", l.lrc.len(), l.tlrc.len());
    println!("resolve: {:?}", link::resolve(&api, "https://music.163.com/#/album?id=18905").await?);
    let q = api.qr_start().await?;
    println!("qr: key {} svg {} bytes; check {:?}", q.key.len(), q.svg.len(), api.qr_check(&q.key).await?.0);
    // 找一首免费歌曲测试下载
    let free = p.tracks.iter().find(|t| t.fee == 0 || t.fee == 8).unwrap().clone();
    match api.song_url(free.id, "exhigh").await {
        Ok(u) => println!("url: {} {} {}", u.level, u.format, u.size),
        Err(e) => println!("url err: {e}"),
    }
    let item = DownloadItem { track: free, index: Some(1), total: Some(10) };
    let opts = DownloadOptions { lrc_file: true, ..Default::default() };
    let d = download(&api, &item, &opts, std::path::Path::new("/tmp/opencode/dl"), &Cancel::default(), |_, _| {}).await?;
    println!("downloaded: {:?}", d);
    Ok(())
}
