//! 统一的只读数据缓存。所有音源共用一层，按数据易变性分三档 TTL；
//! 播放地址、账号、二维码等带签名或依赖登录态的数据不进入这里。
//!
//! 使用 moka 的 `get_with`：同一 key 的并发请求只有一个会真正打上游，
//! 其余等待并复用结果（避免首页刷新时多个组件同时拉同一份数据）。

use std::sync::Arc;
use std::time::Duration;

use moka::future::Cache;

use crate::model::{Collection, HomeFeed, Lyric, Track};

/// 内容发布后不再变化：歌词、歌曲详情、专辑
const TTL_IMMUTABLE: Duration = Duration::from_secs(24 * 3600);
/// 偶尔变化：歌单、榜单、搜索结果
const TTL_SLOW: Duration = Duration::from_secs(10 * 60);
/// 首页推荐：平台每日更新，但用户期望刷新后尽快看到新内容
const TTL_HOME: Duration = Duration::from_secs(15 * 60);

/// 各类缓存条目上限。条目体积：歌词 ~10KB、歌单 ~数百 KB（大歌单）、搜索 ~30KB。
const CAP_LYRIC: u64 = 2000;
const CAP_TRACK: u64 = 5000;
const CAP_COLLECTION: u64 = 300;
const CAP_SEARCH: u64 = 500;
const CAP_HOME: u64 = 8;

#[derive(Clone)]
pub struct DataCache {
    lyric: Cache<String, Arc<Lyric>>,
    track: Cache<String, Arc<Track>>,
    /// 歌单 / 专辑 / 榜单共用，key 含 kind
    collection: Cache<String, Arc<Collection>>,
    search: Cache<String, Arc<Vec<Track>>>,
    home: Cache<String, Arc<HomeFeed>>,
}

impl Default for DataCache {
    fn default() -> Self {
        Self::new()
    }
}

impl DataCache {
    pub fn new() -> Self {
        Self {
            lyric: Cache::builder().max_capacity(CAP_LYRIC).time_to_live(TTL_IMMUTABLE).build(),
            track: Cache::builder().max_capacity(CAP_TRACK).time_to_live(TTL_IMMUTABLE).build(),
            collection: Cache::builder().max_capacity(CAP_COLLECTION).time_to_live(TTL_SLOW).build(),
            search: Cache::builder().max_capacity(CAP_SEARCH).time_to_live(TTL_SLOW).build(),
            home: Cache::builder().max_capacity(CAP_HOME).time_to_live(TTL_HOME).build(),
        }
    }

    pub async fn lyric<F, Fut>(&self, source: &str, id: &str, load: F) -> crate::error::Result<Arc<Lyric>>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = crate::error::Result<Lyric>>,
    {
        self.lyric.try_get_with(format!("{source}:{id}"), async { load().await.map(Arc::new) }).await.map_err(arc_err)
    }

    pub async fn track<F, Fut>(&self, source: &str, id: &str, load: F) -> crate::error::Result<Arc<Track>>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = crate::error::Result<Track>>,
    {
        self.track.try_get_with(format!("{source}:{id}"), async { load().await.map(Arc::new) }).await.map_err(arc_err)
    }

    pub async fn collection<F, Fut>(&self, source: &str, kind: &str, id: &str, load: F) -> crate::error::Result<Arc<Collection>>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = crate::error::Result<Collection>>,
    {
        self.collection
            .try_get_with(format!("{source}:{kind}:{id}"), async { load().await.map(Arc::new) })
            .await
            .map_err(arc_err)
    }

    /// 搜索结果缓存。key 含音源、关键词（小写去空白）、分页参数。
    pub async fn search<F, Fut>(&self, source: &str, q: &str, limit: u32, offset: u32, load: F) -> crate::error::Result<Arc<Vec<Track>>>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = crate::error::Result<Vec<Track>>>,
    {
        let key = format!("{source}:{limit}:{offset}:{}", q.trim().to_lowercase());
        self.search.try_get_with(key, async { load().await.map(Arc::new) }).await.map_err(arc_err)
    }

    pub async fn home<F, Fut>(&self, source: &str, load: F) -> crate::error::Result<Arc<HomeFeed>>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = crate::error::Result<HomeFeed>>,
    {
        self.home.try_get_with(source.to_owned(), async { load().await.map(Arc::new) }).await.map_err(arc_err)
    }

    /// 登录态变化后调用：曲目的 `availability`、歌单可见性、首页推荐都随账号变。
    /// 歌词是纯内容，不受影响，保留。
    pub fn invalidate_account_scoped(&self) {
        self.track.invalidate_all();
        self.collection.invalidate_all();
        self.search.invalidate_all();
        self.home.invalidate_all();
    }

    pub fn clear(&self) {
        self.lyric.invalidate_all();
        self.invalidate_account_scoped();
    }
}

/// moka 把加载错误包成 `Arc<E>`，我们的 Error 不是 Clone，取出时转成字符串错误
fn arc_err(e: Arc<crate::error::Error>) -> crate::error::Error {
    match Arc::try_unwrap(e) {
        Ok(inner) => inner,
        Err(shared) => crate::error::Error::Upstream(shared.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[tokio::test]
    async fn coalesces_concurrent_loads_and_caches() {
        let c = DataCache::new();
        let calls = Arc::new(AtomicUsize::new(0));
        let load = || {
            let calls = calls.clone();
            async move {
                calls.fetch_add(1, Ordering::SeqCst);
                tokio::time::sleep(Duration::from_millis(30)).await;
                Ok(Lyric { lrc: "[00:00.00]hi".into(), tlrc: String::new() })
            }
        };
        // 五个并发请求同一 key：只应加载一次
        let r = futures_util::future::join_all((0..5).map(|_| c.lyric("netease", "1", load))).await;
        assert!(r.iter().all(Result::is_ok));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        // 再次请求命中缓存，不再加载
        c.lyric("netease", "1", load).await.unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        // 不同音源同 id 是不同 key
        c.lyric("qq", "1", load).await.unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn errors_are_not_cached() {
        let c = DataCache::new();
        let calls = Arc::new(AtomicUsize::new(0));
        let load = || {
            let calls = calls.clone();
            async move {
                calls.fetch_add(1, Ordering::SeqCst);
                Err::<Lyric, _>(crate::error::Error::Unavailable)
            }
        };
        assert!(c.lyric("netease", "x", load).await.is_err());
        assert!(c.lyric("netease", "x", load).await.is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 2, "失败不应被缓存");
    }

    #[tokio::test]
    async fn credential_change_keeps_lyrics_but_drops_tracks() {
        let c = DataCache::new();
        let n = Arc::new(AtomicUsize::new(0));
        let mk_track = || {
            let n = n.clone();
            async move {
                n.fetch_add(1, Ordering::SeqCst);
                Ok(Track {
                    id: "1".into(), source: "netease".into(), name: "a".into(), alias: None, artists: vec![],
                    album_id: String::new(), album: String::new(), cover: String::new(), duration: 0, fee: 0,
                    availability: Default::default(),
                    isrc: None,
                })
            }
        };
        let mk_lyric = || {
            let n = n.clone();
            async move {
                n.fetch_add(1, Ordering::SeqCst);
                Ok(Lyric::default())
            }
        };
        c.track("netease", "1", mk_track).await.unwrap();
        c.lyric("netease", "1", mk_lyric).await.unwrap();
        assert_eq!(n.load(Ordering::SeqCst), 2);
        c.invalidate_account_scoped();
        c.track("netease", "1", mk_track).await.unwrap(); // 重新加载
        c.lyric("netease", "1", mk_lyric).await.unwrap(); // 仍命中
        assert_eq!(n.load(Ordering::SeqCst), 3);
    }
}
