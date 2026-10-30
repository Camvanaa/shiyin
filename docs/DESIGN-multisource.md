# 多音源适配设计（v0.3）

参考本地 `ML-Music_url/providers/` 的适配器思路，把拾音从“网易云专用”扩展为“播放器 + 多音源”。

## 原则

- 保持精简：不引入插件脚本沙箱，音源是编译进二进制的 Rust 实现。
- 统一模型：任何平台都映射到同一套 `Track / Collection / SongUrl / Lyric`。
- 统一音质：`standard / exhigh / lossless / hires`，平台不支持时降级，以实际返回为准。
- 能力声明：每个音源声明支持的音质、是否能扫码登录、是否有歌单/专辑，前端据此显示。

## 模型泛化

- `Track.id: String`，新增 `Track.source: String`（如 `netease` / `qq` / `kuwo`）。
- `Track.album_id: String`，`Collection.id: String`，`Collection.source: String`。
- `Resolved` 携带 `source`，链接识别按音源分发。

## Source trait（core::source）

```rust
trait Source {
    fn id(&self) -> &'static str;
    fn label(&self) -> &'static str;
    fn levels(&self) -> &'static [&'static str];
    fn http(&self) -> &reqwest::Client;
    fn parse_link(&self, text: &str) -> Option<Resolved>;
    fn search(...) -> BoxFut<Vec<Track>>;
    fn track(&self, id: &str) -> BoxFut<Track>;
    fn collection(&self, kind: CollectionKind, id: &str) -> BoxFut<Collection>;
    fn song_url(&self, id: &str, level: &str) -> BoxFut<SongUrl>;
    fn lyric(&self, id: &str) -> BoxFut<Lyric>;
    fn sized_cover(&self, url: &str, size: u32) -> String;
    fn is_allowed_media_url(&self, url: &str) -> bool;
}
```

`Sources` 注册表按 `track.source` 路由到对应实现；下载与任务调度只依赖注册表。

## 首批音源

| 音源 | 依据 | 说明 |
| --- | --- | --- |
| netease | 已有 | 保留全部能力（扫码登录、eapi、榜单/推荐） |
| kuwo | ML-Music_url/providers/kuwo.py | 公开接口、无加密，最先落地 |
| qq | ML-Music_url/providers/qq.py | GET `CgiGetVkey`，无需签名 |
| kugou | 暂缓 | 平台风控验证码，不适合自动解析 |

## 前端

- 搜索框左侧音源选择器（全部 / 网易云 / QQ / 酷我）。
- 歌曲行显示来源徽标；播放、下载按 `track.source` 路由。
- 封面裁剪参数由音源决定（网易云 `?param=`，QQ 用固定尺寸 CDN，酷我原图）。
