//! 托盘、桌面歌词窗口、迷你播放器、全局快捷键。
//!
//! 播放只在主窗口的 WebView 里进行。子窗口按需创建，关闭即销毁，
//! 不常驻内存。托盘菜单和快捷键通过 `tray` 事件转发给主窗口。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use serde::Deserialize;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, Runtime, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent,
    Wry,
};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

const LYRICS: &str = "lyrics";
const MINI: &str = "mini";

#[derive(Default)]
pub struct DesktopState {
    close_to_tray: AtomicBool,
    shortcuts: AtomicBool,
    locked: AtomicBool,
    /// 程序正在退出，此时关闭主窗口不再拦截
    quitting: AtomicBool,
    menu: Mutex<Option<TrayMenu>>,
}

struct TrayMenu {
    lyrics: CheckMenuItem<Wry>,
    lock: CheckMenuItem<Wry>,
    mini: CheckMenuItem<Wry>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Prefs {
    close_to_tray: bool,
    global_shortcuts: bool,
}

fn shortcuts() -> [(Shortcut, &'static str); 4] {
    let m = Some(Modifiers::CONTROL | Modifiers::ALT);
    [
        (Shortcut::new(m, Code::KeyP), "toggle"),
        (Shortcut::new(m, Code::ArrowLeft), "prev"),
        (Shortcut::new(m, Code::ArrowRight), "next"),
        (Shortcut::new(m, Code::KeyL), "lyrics"),
    ]
}

pub fn plugin_global_shortcut() -> tauri::plugin::TauriPlugin<Wry> {
    tauri_plugin_global_shortcut::Builder::new()
        .with_handler(|app, sc, ev| {
            if ev.state() != ShortcutState::Pressed {
                return;
            }
            if let Some((_, action)) = shortcuts().into_iter().find(|(s, _)| s == sc) {
                forward(app, action);
            }
        })
        .build()
}

/// 把托盘或快捷键动作交给主窗口执行
fn forward<R: Runtime>(app: &AppHandle<R>, action: &str) {
    let _ = app.emit_to("main", "tray", action);
}

pub fn show_main<R: Runtime>(app: &AppHandle<R>) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

pub fn setup(app: &tauri::App) -> tauri::Result<()> {
    let state = DesktopState::default();
    state.close_to_tray.store(true, Ordering::Relaxed);

    let show = MenuItem::with_id(app, "show", "显示拾音", true, None::<&str>)?;
    let toggle = MenuItem::with_id(app, "toggle", "播放 / 暂停", true, None::<&str>)?;
    let prev = MenuItem::with_id(app, "prev", "上一首", true, None::<&str>)?;
    let next = MenuItem::with_id(app, "next", "下一首", true, None::<&str>)?;
    let lyrics = CheckMenuItem::with_id(app, "lyrics", "桌面歌词", true, false, None::<&str>)?;
    let lock = CheckMenuItem::with_id(app, "lock", "锁定桌面歌词", true, false, None::<&str>)?;
    let mini = CheckMenuItem::with_id(app, "mini", "迷你播放器", true, false, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let sep = || PredefinedMenuItem::separator(app);
    let menu = Menu::with_items(
        app,
        &[&show, &sep()?, &toggle, &prev, &next, &sep()?, &lyrics, &lock, &mini, &sep()?, &quit],
    )?;

    TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().cloned().expect("icon"))
        .tooltip("拾音")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, e| match e.id().as_ref() {
            "show" => show_main(app),
            "quit" => {
                app.state::<DesktopState>().quitting.store(true, Ordering::Relaxed);
                app.exit(0);
            }
            "lyrics" => {
                let on = app.get_webview_window(LYRICS).is_none();
                let _ = set_lyrics(app, on);
            }
            "lock" => {
                let locked = !app.state::<DesktopState>().locked.load(Ordering::Relaxed);
                let _ = set_locked(app, locked);
            }
            "mini" => {
                let on = app.get_webview_window(MINI).is_none();
                let _ = set_mini(app, on);
            }
            other => forward(app, other),
        })
        .on_tray_icon_event(|tray, e| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = e {
                let app = tray.app_handle();
                if app.get_webview_window(MINI).is_some() {
                    let _ = set_mini(app, false);
                }
                show_main(app);
            }
        })
        .build(app)?;

    *state.menu.lock().unwrap() = Some(TrayMenu { lyrics, lock, mini });
    app.manage(state);

    // 关闭主窗口时隐藏到托盘
    if let Some(w) = app.get_webview_window("main") {
        let handle = app.handle().clone();
        w.on_window_event(move |e| {
            if let WindowEvent::CloseRequested { api, .. } = e {
                let st = handle.state::<DesktopState>();
                if st.close_to_tray.load(Ordering::Relaxed) && !st.quitting.load(Ordering::Relaxed) {
                    api.prevent_close();
                    if let Some(w) = handle.get_webview_window("main") {
                        let _ = w.hide();
                    }
                } else {
                    // 真正退出时一并关闭子窗口
                    for l in [LYRICS, MINI] {
                        if let Some(w) = handle.get_webview_window(l) {
                            let _ = w.destroy();
                        }
                    }
                }
            }
        });
    }
    Ok(())
}

fn sync<R: Runtime>(app: &AppHandle<R>) {
    let st = app.state::<DesktopState>();
    let lyrics = app.get_webview_window(LYRICS).is_some();
    let mini = app.get_webview_window(MINI).is_some();
    let locked = st.locked.load(Ordering::Relaxed);
    if let Some(m) = st.menu.lock().unwrap().as_ref() {
        let _ = m.lyrics.set_checked(lyrics);
        let _ = m.lock.set_checked(locked);
        let _ = m.lock.set_enabled(lyrics);
        let _ = m.mini.set_checked(mini);
    }
    let _ = app.emit_to(
        "main",
        "windows",
        serde_json::json!({ "lyrics": lyrics, "locked": locked, "mini": mini }),
    );
}

/// 窗口位置在当前任意显示器内才使用，否则居中（防止拔掉副屏后窗口跑到屏幕外）
fn place_saved<R: Runtime>(w: &WebviewWindow<R>, key: &str) {
    let Some(pos) = load_pos(w.app_handle(), key) else { return };
    let ok = w.available_monitors().ok().is_some_and(|ms| {
        ms.iter().any(|m| {
            let (p, s) = (m.position(), m.size());
            pos.x >= p.x - 40
                && pos.y >= p.y - 10
                && pos.x < p.x + s.width as i32 - 80
                && pos.y < p.y + s.height as i32 - 40
        })
    });
    if ok {
        let _ = w.set_position(pos);
    }
}

fn pos_file<R: Runtime>(app: &AppHandle<R>) -> Option<std::path::PathBuf> {
    app.path().app_data_dir().ok().map(|d| d.join("windows.json"))
}

fn load_pos<R: Runtime>(app: &AppHandle<R>, key: &str) -> Option<PhysicalPosition<i32>> {
    let v: serde_json::Value = serde_json::from_slice(&std::fs::read(pos_file(app)?).ok()?).ok()?;
    Some(PhysicalPosition::new(v[key]["x"].as_i64()? as i32, v[key]["y"].as_i64()? as i32))
}

fn save_pos<R: Runtime>(app: &AppHandle<R>, key: &str, p: PhysicalPosition<i32>) {
    let Some(f) = pos_file(app) else { return };
    let mut v: serde_json::Value = std::fs::read(&f)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_else(|| serde_json::json!({}));
    v[key] = serde_json::json!({ "x": p.x, "y": p.y });
    if let Some(d) = f.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    let _ = std::fs::write(f, v.to_string());
}

fn track_position<R: Runtime>(w: &WebviewWindow<R>, key: &'static str) {
    let app = w.app_handle().clone();
    w.on_window_event(move |e| match e {
        WindowEvent::Moved(p) => save_pos(&app, key, *p),
        WindowEvent::Destroyed => {
            let app2 = app.clone();
            // 销毁事件触发时窗口仍在注册表中，稍后再同步状态
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                sync(&app2);
            });
        }
        _ => {}
    });
}

pub fn set_lyrics<R: Runtime>(app: &AppHandle<R>, on: bool) -> tauri::Result<()> {
    let existing = app.get_webview_window(LYRICS);
    match (on, existing) {
        (true, None) => {
            let w = WebviewWindowBuilder::new(app, LYRICS, WebviewUrl::App("lyrics.html".into()))
                .title("拾音 · 桌面歌词")
                .inner_size(820.0, 136.0)
                .min_inner_size(360.0, 96.0)
                .decorations(false)
                .transparent(true)
                .shadow(false)
                .always_on_top(true)
                .skip_taskbar(true)
                .resizable(true)
                .focused(false)
                .visible(false)
                .build()?;
            // 默认放在主屏幕底部居中
            if load_pos(app, LYRICS).is_none() {
                if let Ok(Some(m)) = w.primary_monitor() {
                    let (p, s) = (m.position(), m.size());
                    let ws = w.outer_size()?;
                    let _ = w.set_position(PhysicalPosition::new(
                        p.x + (s.width as i32 - ws.width as i32) / 2,
                        p.y + s.height as i32 - ws.height as i32 - (s.height as i32 / 10),
                    ));
                }
            }
            place_saved(&w, LYRICS);
            track_position(&w, LYRICS);
            let locked = app.state::<DesktopState>().locked.load(Ordering::Relaxed);
            let _ = w.set_ignore_cursor_events(locked);
            w.show()?;
        }
        (false, Some(w)) => w.destroy()?,
        _ => {}
    }
    sync(app);
    Ok(())
}

pub fn set_locked<R: Runtime>(app: &AppHandle<R>, locked: bool) -> tauri::Result<()> {
    app.state::<DesktopState>().locked.store(locked, Ordering::Relaxed);
    if let Some(w) = app.get_webview_window(LYRICS) {
        w.set_ignore_cursor_events(locked)?;
        let _ = w.emit("lyrics:locked", serde_json::json!({ "locked": locked }));
    }
    sync(app);
    Ok(())
}

pub fn set_mini<R: Runtime>(app: &AppHandle<R>, on: bool) -> tauri::Result<()> {
    let existing = app.get_webview_window(MINI);
    match (on, existing) {
        (true, None) => {
            let w = WebviewWindowBuilder::new(app, MINI, WebviewUrl::App("mini.html".into()))
                .title("拾音")
                .inner_size(360.0, 88.0)
                .resizable(false)
                .maximizable(false)
                .decorations(false)
                .transparent(true)
                .shadow(true)
                .always_on_top(true)
                .skip_taskbar(false)
                .visible(false)
                .build()?;
            if load_pos(app, MINI).is_none() {
                if let Ok(Some(m)) = w.primary_monitor() {
                    let (p, s) = (m.position(), m.size());
                    let ws = w.outer_size()?;
                    let margin = (24.0 * m.scale_factor()) as i32;
                    let _ = w.set_position(PhysicalPosition::new(
                        p.x + s.width as i32 - ws.width as i32 - margin,
                        p.y + s.height as i32 - ws.height as i32 - margin * 3,
                    ));
                }
            }
            place_saved(&w, MINI);
            track_position(&w, MINI);
            w.show()?;
            if let Some(main) = app.get_webview_window("main") {
                let _ = main.hide();
            }
        }
        (false, Some(w)) => {
            show_main(app);
            w.destroy()?;
        }
        _ => {}
    }
    sync(app);
    Ok(())
}

// ---------- 命令 ----------

#[tauri::command]
pub fn set_lyrics_window(app: AppHandle, on: bool) -> Result<(), String> {
    set_lyrics(&app, on).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_lyrics_locked(app: AppHandle, locked: bool) -> Result<(), String> {
    set_locked(&app, locked).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_mini_window(app: AppHandle, on: bool) -> Result<(), String> {
    set_mini(&app, on).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_desktop_prefs(app: AppHandle, prefs: Prefs) -> Result<(), String> {
    let st = app.state::<DesktopState>();
    st.close_to_tray.store(prefs.close_to_tray, Ordering::Relaxed);
    let was = st.shortcuts.swap(prefs.global_shortcuts, Ordering::Relaxed);
    if was != prefs.global_shortcuts {
        let gs = app.global_shortcut();
        for (sc, _) in shortcuts() {
            let r = if prefs.global_shortcuts { gs.register(sc) } else { gs.unregister(sc) };
            // 被其他程序占用时忽略，不影响其余快捷键
            if let Err(e) = r {
                eprintln!("快捷键 {sc:?}：{e}");
            }
        }
    }
    Ok(())
}
