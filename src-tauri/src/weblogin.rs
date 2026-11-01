//! 桌面版应用内网页登录：在独立 webview 窗口里打开平台登录页，用户在窗口内
//! 自行登录（密码不经过本应用的任何代码），登录完成后从 webview 的 cookie
//! 存储里读取目标 cookie 并关闭窗口。
//!
//! 用途：Apple Music 的 `media-user-token` 属于 httpOnly cookie，用户手动去
//! 开发者工具里复制很麻烦；用这个窗口登录后自动取。任何平台的 cookie 都能用。

use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

/// 登录窗口的 label
const LABEL: &str = "web-login";

#[derive(Serialize, Clone)]
struct LoginEvent {
    source: String,
    /// "done" | "cancelled" | "error"
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    cookie: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

/// 在 webview 里打开 `url` 登录，等待名为 `marker` 的 cookie 出现（登录态标志），然后：
/// - `whole = false`：只返回该 cookie 的值（如 Apple 的 media-user-token）
/// - `whole = true`：返回该域名下的整段 cookie（如酷我，后端把整段 cookie 作请求头转发）
///
/// 窗口被用户关闭则返回错误。
///
/// 必须在 async 命令里调用：Windows 上同步读取 cookie 会与 WebView2 事件循环死锁。
pub async fn webview_login(app: AppHandle, url: String, marker: String, whole: bool, domain: String, source: String) -> Result<String, String> {
    let parsed = tauri::Url::parse(&url).map_err(|e| format!("登录地址无效：{e}"))?;

    // 已存在同名窗口时先关掉，避免复用旧会话
    if let Some(w) = app.get_webview_window(LABEL) {
        let _ = w.close();
        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    let title = format!("登录 {source}");
    let window = WebviewWindowBuilder::new(&app, LABEL, WebviewUrl::External(parsed))
        .title(title)
        .inner_size(520.0, 720.0)
        .min_inner_size(380.0, 520.0)
        .center()
        .build()
        .map_err(|e| format!("无法打开登录窗口：{e}"))?;

    let cookie_domain = domain.clone();
    let marker_owned = marker.clone();
    let source_owned = source.clone();
    let app2 = app.clone();
    let win = window.clone();

    // 轮询 cookie：cookies_for_url 会阻塞，放到独立阻塞线程
    let result = tokio::task::spawn_blocking(move || -> Result<String, String> {
        let url = tauri::Url::parse(&format!("https://{}", cookie_domain.trim_start_matches("https://").trim_end_matches('/')))
            .map_err(|e| format!("域名无效：{e}"))?;
        let deadline = std::time::Instant::now() + Duration::from_secs(300);
        loop {
            // 窗口被用户关掉 → 取消
            if app2.get_webview_window(LABEL).is_none() {
                return Err("已取消登录".into());
            }
            if let Ok(cookies) = win.cookies_for_url(url.clone()) {
                if let Some(c) = cookies.iter().find(|c| c.name() == marker_owned && !c.value().is_empty()) {
                    if whole {
                        let all: Vec<String> = cookies.iter().filter(|c| !c.value().is_empty()).map(|c| format!("{}={}", c.name(), c.value())).collect();
                        return Ok(all.join("; "));
                    }
                    return Ok(c.value().to_owned());
                }
            }
            if std::time::Instant::now() > deadline {
                return Err("登录超时，请重试".into());
            }
            std::thread::sleep(Duration::from_millis(800));
        }
    })
    .await
    .map_err(|e| format!("登录任务异常：{e}"))?;

    // 收尾：关闭窗口并广播结果
    if let Some(w) = app.get_webview_window(LABEL) {
        let _ = w.close();
    }
    match &result {
        Ok(c) => {
            let _ = app.emit(
                "web-login://done",
                LoginEvent { source: source_owned, status: "done".into(), cookie: Some(c.clone()), message: None },
            );
        }
        Err(m) => {
            let status = if m.contains("取消") { "cancelled" } else { "error" };
            let _ = app.emit(
                "web-login://done",
                LoginEvent { source: source_owned, status: status.into(), cookie: None, message: Some(m.clone()) },
            );
        }
    }
    result
}

/// 取消正在进行的网页登录（关闭窗口）
pub fn webview_login_cancel(app: AppHandle) {
    if let Some(w) = app.get_webview_window(LABEL) {
        let _ = w.close();
    }
}
