# 拾音 (Shiyin)

轻量、优雅的多平台音乐播放与下载工具。基于 Rust + Svelte 5 + Tauri v2 构建，提供 **Windows 桌面版**、**Android 手机版** 和 **可自托管 Web 版**。遵循 Apple 设计风格，界面克制简洁、动效细腻丝滑。

## 🌟 核心特性

- **多音源深度融合**：同时支持网易云音乐、QQ 音乐、酷我音乐。可在主页直接切换独立平台或全景【聚合】浏览。
- **智能自动换源**：当曲目在当前平台遇到版权缺失或仅限试听时，播放引擎自动在其他平台检索同名可播音源，无缝续播。
- **全平台一致体验**：
  - **Windows 桌面版**：独立 Rust 本地内核，支持系统托盘（关闭进托盘后台常驻）、透明置顶桌面歌词（支持鼠标穿透锁定）、迷你播放器小窗口、全局快捷键。
  - **Android 手机版**：Tauri 2 移动端原生打包，支持触摸全面屏手势、安全区沉浸适配、系统通知栏与锁屏音乐控制。
  - **Web 预览版**：单二进制内嵌静态资源的 HTTP 服务，开箱自托管。
- **全音源凭证管理**：支持网易云手机扫码登录 / MUSIC_U，以及 QQ 音乐与酷我 Cookie 配置。
- **高品质解析与下载**：支持标准、高品、无损 (FLAC) 音质下载；自动写入 ID3 / Vorbis 元数据、专辑封面、内嵌歌词与外置 `.lrc` 文件。

## 📥 下载与安装

进入项目的 [GitHub Actions](../../actions) 页面：
- **Windows 安装包**：在 `build` 工作流中下载 `shiyin-windows` 产物（解压即用安装程序）。
- **Android APK**：在 `android` 工作流中下载 `shiyin-android-apk` 产物（安装到 Android 手机）。

## 📁 架构与目录

```
crates/
  core/           统一模型、音源适配器 (网易/QQ/酷我)、下载引擎、元数据写入
  server/         Web 版 axum HTTP 服务，单二进制内嵌前端
src-tauri/        Tauri v2 跨平台外壳 (Windows / Android)
web/              Svelte 5 + Vite 前端，Apple 设计规范，三端共用
docs/             设计与架构演进文档
```

## 🛠️ 本地开发

```bash
cd web && npm ci
npm run build                     # 编译前端

# 启动 Web 版
cargo run -p shiyin-server        # http://127.0.0.1:6970

# 启动 Windows 桌面版开发模式
cd src-tauri && ../web/node_modules/.bin/tauri dev
```

## 📄 开源许可

GPL-3.0 协议。
