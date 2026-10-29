//! 音源适配器接口。下载、任务调度与具体平台解耦；网易云是默认适配器。

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::api::Netease;
use crate::error::Result;
use crate::model::{Collection, CollectionKind, Lyric, Resolved, SongUrl, Track};

pub type BoxFut<'a, T> = Pin<Box<dyn Future<Output = Result<T>> + Send + 'a>>;

pub trait Source: Send + Sync {
    fn id(&self) -> &'static str;

    fn http(&self) -> &reqwest::Client;

    fn resolve<'a>(&'a self, text: &'a str) -> BoxFut<'a, Resolved>;

    fn search<'a>(&'a self, keywords: &'a str, limit: u32, offset: u32) -> BoxFut<'a, Vec<Track>>;

    fn tracks<'a>(&'a self, ids: &'a [u64]) -> BoxFut<'a, Vec<Track>>;

    fn collection(&self, kind: CollectionKind, id: u64) -> BoxFut<'_, Collection>;

    fn song_url<'a>(&'a self, id: u64, level: &'a str) -> BoxFut<'a, SongUrl>;

    fn lyric(&self, id: u64) -> BoxFut<'_, Lyric>;

    /// 根据显示尺寸返回合适的封面地址。默认原样返回。
    fn sized_cover(&self, url: &str, _size: u32) -> String {
        url.to_owned()
    }

    /// 音频和封面地址域名白名单检查，防止服务端被当作开放代理。
    fn is_allowed_media_url(&self, url: &str) -> bool;
}

pub type DynSource = Arc<dyn Source>;

impl Source for Netease {
    fn id(&self) -> &'static str {
        "netease"
    }

    fn http(&self) -> &reqwest::Client {
        Netease::http(self)
    }

    fn resolve<'a>(&'a self, text: &'a str) -> BoxFut<'a, Resolved> {
        Box::pin(crate::link::resolve(self, text))
    }

    fn search<'a>(&'a self, keywords: &'a str, limit: u32, offset: u32) -> BoxFut<'a, Vec<Track>> {
        Box::pin(Netease::search(self, keywords, limit, offset))
    }

    fn tracks<'a>(&'a self, ids: &'a [u64]) -> BoxFut<'a, Vec<Track>> {
        Box::pin(Netease::song_details(self, ids))
    }

    fn collection(&self, kind: CollectionKind, id: u64) -> BoxFut<'_, Collection> {
        Box::pin(async move {
            match kind {
                CollectionKind::Playlist => self.playlist(id).await,
                CollectionKind::Album => self.album(id).await,
            }
        })
    }

    fn song_url<'a>(&'a self, id: u64, level: &'a str) -> BoxFut<'a, SongUrl> {
        Box::pin(Netease::song_url(self, id, level))
    }

    fn lyric(&self, id: u64) -> BoxFut<'_, Lyric> {
        Box::pin(Netease::lyric(self, id))
    }

    fn sized_cover(&self, url: &str, size: u32) -> String {
        crate::download::sized_cover(url, size)
    }

    fn is_allowed_media_url(&self, url: &str) -> bool {
        crate::download::is_allowed_media_url(url)
    }
}
