# 拾音

网易云音乐搜索、播放与下载工具。基于 [ML-Netease_url](https://github.com/midairlogn/ML-Netease_url) 用 Rust + Svelte 重写，提供 Windows 桌面版（Tauri 2）和可自托管的 Web 版。

## 功能

- 一个输入框完成搜索，粘贴歌曲、歌单、专辑链接（含 163cn.tv 短链）直接打开
- 在线播放：播放队列、随机和循环、逐行歌词（含翻译）、系统媒体键
- 下载：可选音质，写入标题、歌手、专辑、曲目号、封面、歌词，可另存 .lrc，支持文件名模板
- 下载管理：并发控制、暂停、继续、取消、失败重试
- 扫码登录，或手动填写 MUSIC_U。凭证只保存在本机（Web 版保存在服务端数据目录）

## 下载桌面版

在 [Actions](../../actions) 中打开最新一次成功的构建，下载 `shiyin-windows` 产物。打 `v*` 标签时会自动创建草稿 Release。

## 目录

```
crates/core     网易云接口、eapi 加密、元数据写入、下载任务管理
crates/server   Web 版：axum HTTP 服务，内嵌前端
src-tauri       桌面版外壳
web             Svelte 5 前端（两种形态共用）
docs/DESIGN.md  设计文档
```

## 开发

```bash
cd web && npm ci
npm run build                     # 构建前端到 web/dist

# Web 版
cargo run -p shiyin-server        # http://127.0.0.1:6970

# 桌面版
cd src-tauri && ../web/node_modules/.bin/tauri dev
```

## Web 版部署

| 环境变量 | 说明 | 默认值 |
| --- | --- | --- |
| `SHIYIN_ADDR` | 监听地址 | `127.0.0.1:6970` |
| `SHIYIN_TOKEN` | 访问口令。监听非本机地址时必须设置 | 无 |
| `SHIYIN_DATA_DIR` | 凭证和临时文件目录 | `~/.local/share/shiyin` |
| `MUSIC_U` | 初始凭证（可选，也可在界面中登录） | 无 |

## 许可

GPL-3.0，与上游项目一致。
