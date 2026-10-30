#[cfg(desktop)]
use desktop::{set_desktop_prefs, set_lyrics_locked, set_lyrics_window, set_mini_window};
#[cfg(desktop)]
mod desktop;

use std::path::PathBuf;

use serde::Deserialize;
use serde_json::{json, Value};
use shiyin_core::download::{DownloadItem, DownloadOptions};
use shiyin_core::jobs::{JobManager, JobSnapshot, JobSummary, NewJob};
use shiyin_core::store::CredentialStore;
use shiyin_core::{Account, Collection, CollectionKind, Lyric, Netease, QrStart, Resolved, SongUrl, Sources, Track};
use tauri::{Emitter, Manager, State};

struct Ctx {
    api: Netease,
    sources: Sources,
    store: CredentialStore,
    jobs: JobManager,
}

type R<T> = Result<T, String>;

fn pick<'a>(s: &'a State<'a, Ctx>, source: &str) -> Result<&'a shiyin_core::DynSource, String> {
    let key = if source.is_empty() { "netease" } else { source };
    s.sources.get(key).ok_or_else(|| format!("未知音源：{key}"))
}

fn e<E: ToString>(err: E) -> String {
    err.to_string()
}

#[tauri::command]
async fn home(s: State<'_, Ctx>, source: Option<String>) -> R<shiyin_core::HomeFeed> {
    s.sources.home(source.as_deref().unwrap_or("")).await.map_err(e)
}

#[tauri::command]
async fn resolve(s: State<'_, Ctx>, q: String) -> R<Resolved> {
    s.sources.resolve(&q).await.map_err(e)
}

#[tauri::command]
fn sources(s: State<'_, Ctx>) -> Vec<serde_json::Value> {
    s.sources
        .list()
        .iter()
        .map(|x| serde_json::json!({ "id": x.id(), "label": x.label(), "levels": x.levels(), "hasCookie": x.has_cookie() }))
        .collect()
}

#[tauri::command]
async fn search(s: State<'_, Ctx>, q: String, limit: u32, offset: u32, source: String) -> R<Vec<Track>> {
    if source.is_empty() || source == "all" {
        return Ok(s.sources.search_all(&q, limit).await);
    }
    pick(&s, &source)?.search(&q, limit, offset).await.map_err(e)
}

#[tauri::command]
async fn song(s: State<'_, Ctx>, id: String, source: String) -> R<Track> {
    pick(&s, &source)?.track(&id).await.map_err(e)
}

#[tauri::command]
async fn song_url(s: State<'_, Ctx>, id: String, level: String, source: String) -> R<SongUrl> {
    pick(&s, &source)?.song_url(&id, &level).await.map_err(e)
}

#[tauri::command]
async fn lyric(s: State<'_, Ctx>, id: String, source: String) -> R<Lyric> {
    pick(&s, &source)?.lyric(&id).await.map_err(e)
}

#[tauri::command]
async fn playlist(s: State<'_, Ctx>, id: String, source: String) -> R<Collection> {
    pick(&s, &source)?.collection(CollectionKind::Playlist, &id).await.map_err(e)
}

#[tauri::command]
async fn album(s: State<'_, Ctx>, id: String, source: String) -> R<Collection> {
    pick(&s, &source)?.collection(CollectionKind::Album, &id).await.map_err(e)
}

#[tauri::command]
async fn toplist(s: State<'_, Ctx>, id: String, source: String) -> R<Collection> {
    pick(&s, &source)?.collection(CollectionKind::Toplist, &id).await.map_err(e)
}

#[tauri::command]
async fn match_playable(s: State<'_, Ctx>, name: String, artist: String, skip: String, level: String) -> R<Value> {
    let (track, url) = s
        .sources
        .find_playable(&name, &artist, &skip, &level)
        .await
        .ok_or_else(|| "其它音源也未找到可播放版本".to_owned())?;
    Ok(json!({ "track": track, "url": url }))
}

#[tauri::command]
fn set_source_cookie(s: State<'_, Ctx>, source: String, cookie: Option<String>) -> R<Value> {
    let src = pick(&s, &source)?;
    src.set_cookie(cookie.clone());
    s.store.save_for(&source, cookie.as_deref()).map_err(e)?;
    Ok(json!({ "ok": true, "hasCookie": src.has_cookie() }))
}

#[tauri::command]
async fn account(s: State<'_, Ctx>) -> R<Value> {
    let acc = s.api.account().await.unwrap_or(None);
    Ok(json!({ "account": acc, "hasCredential": s.api.music_u().is_some() }))
}

#[tauri::command]
async fn set_cookie(s: State<'_, Ctx>, music_u: String) -> R<Value> {
    let prev = s.api.music_u();
    s.api.set_music_u(Some(music_u));
    match s.api.account().await {
        Ok(Some(acc)) => {
            s.store.save(s.api.music_u().as_deref()).map_err(e)?;
            Ok(json!({ "account": acc }))
        }
        Ok(None) => {
            s.api.set_music_u(prev);
            Err("MUSIC_U 无效或已过期".into())
        }
        Err(err) => {
            s.api.set_music_u(prev);
            Err(err.to_string())
        }
    }
}

#[tauri::command]
fn logout(s: State<'_, Ctx>) -> R<()> {
    s.api.set_music_u(None);
    s.store.save(None).map_err(e)
}

#[tauri::command]
async fn qr_start(s: State<'_, Ctx>) -> R<QrStart> {
    s.api.qr_start().await.map_err(e)
}

#[tauri::command]
async fn qr_check(s: State<'_, Ctx>, key: String) -> R<Value> {
    let (state, music_u) = s.api.qr_check(&key).await.map_err(e)?;
    let mut account: Option<Account> = None;
    if let Some(mu) = music_u {
        s.api.set_music_u(Some(mu));
        s.store.save(s.api.music_u().as_deref()).map_err(e)?;
        account = s.api.account().await.ok().flatten();
    }
    Ok(json!({ "state": state, "account": account }))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct JobArg {
    title: String,
    #[serde(default)]
    subtitle: String,
    #[serde(default)]
    cover: String,
    items: Vec<DownloadItem>,
    #[serde(default)]
    options: DownloadOptions,
    dir: Option<String>,
}

fn default_dir() -> PathBuf {
    dirs::audio_dir()
        .or_else(dirs::download_dir)
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("拾音")
}

#[tauri::command]
fn add_job(s: State<'_, Ctx>, job: JobArg) -> R<Value> {
    if job.items.is_empty() {
        return Err("没有要下载的歌曲".into());
    }
    if !shiyin_core::is_valid_level(&job.options.level) {
        return Err("未知音质".into());
    }
    let dir = job.dir.filter(|d| !d.trim().is_empty()).map(PathBuf::from).unwrap_or_else(default_dir);
    let id = s.jobs.add(NewJob {
        title: job.title,
        subtitle: job.subtitle,
        cover: job.cover,
        items: job.items,
        opts: job.options,
        dir,
    });
    Ok(json!({ "id": id }))
}

#[tauri::command]
fn jobs(s: State<'_, Ctx>) -> Vec<JobSummary> {
    s.jobs.summaries()
}

#[tauri::command]
fn job(s: State<'_, Ctx>, id: u64) -> R<JobSnapshot> {
    s.jobs.get(id).ok_or_else(|| "任务不存在".into())
}

#[tauri::command]
fn job_action(s: State<'_, Ctx>, id: u64, action: String) -> R<()> {
    match action.as_str() {
        "pause" => s.jobs.pause(id),
        "resume" => s.jobs.resume(id),
        "cancel" => s.jobs.cancel(id),
        "retry" => s.jobs.retry(id),
        _ => return Err("未知操作".into()),
    }
    Ok(())
}

#[tauri::command]
fn remove_job(s: State<'_, Ctx>, id: u64) {
    s.jobs.remove(id);
}

#[tauri::command]
fn clear_jobs(s: State<'_, Ctx>) {
    s.jobs.clear_finished();
}

#[tauri::command]
fn set_concurrency(s: State<'_, Ctx>, value: usize) {
    s.jobs.set_concurrency(value);
}

#[tauri::command]
fn default_dir_cmd() -> String {
    default_dir().to_string_lossy().into_owned()
}

/// 在文件管理器中显示：单曲定位到文件，合集打开目录。
#[tauri::command]
fn reveal(s: State<'_, Ctx>, id: u64) -> R<()> {
    let job = s.jobs.get(id).ok_or("任务不存在")?;
    let files: Vec<_> = job.items.iter().filter_map(|i| i.file.clone()).collect();
    if files.len() == 1 {
        tauri_plugin_opener::reveal_item_in_dir(&files[0]).map_err(e)
    } else {
        tauri_plugin_opener::open_path(&job.dir, None::<&str>).map_err(e)
    }
}


#[cfg(not(desktop))]
#[tauri::command]
fn set_lyrics_window(_app: tauri::AppHandle, _on: bool) -> Result<(), String> { Ok(()) }
#[cfg(not(desktop))]
#[tauri::command]
fn set_lyrics_locked(_app: tauri::AppHandle, _locked: bool) -> Result<(), String> { Ok(()) }
#[cfg(not(desktop))]
#[tauri::command]
fn set_mini_window(_app: tauri::AppHandle, _on: bool) -> Result<(), String> { Ok(()) }
#[cfg(not(desktop))]
#[tauri::command]
fn set_desktop_prefs(_app: tauri::AppHandle, _prefs: serde_json::Value) -> Result<(), String> { Ok(()) }

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init());

    #[cfg(desktop)]
    {
        builder = builder
            .plugin(tauri_plugin_single_instance::init(|app, _, _| {
                desktop::show_main(app);
            }))
            .plugin(
                tauri_plugin_window_state::Builder::default()
                    .with_denylist(&["lyrics", "mini"])
                    .build(),
            )
            .plugin(desktop::plugin_global_shortcut());
    }

    builder
        .setup(|app| {
            let data = app.path().app_data_dir()?;
            let store = CredentialStore::new(&data);
            let api = Netease::new();
            api.set_music_u(store.load());
            let sources = Sources::new(api.clone());
    for src in sources.list() {
        if src.id() != "netease" {
            let env_key = format!("{}_COOKIE", src.id().to_ascii_uppercase());
            src.set_cookie(store.load_for(src.id()).or_else(|| std::env::var(env_key).ok()));
        }
    }
            let jobs = JobManager::new(sources.clone(), 3, None);

            // 任务变化推送给界面，限制为每 200ms 最多一次
            let mut rx = jobs.subscribe();
            let handle = app.handle().clone();
            let jm = jobs.clone();
            tauri::async_runtime::spawn(async move {
                while rx.changed().await.is_ok() {
                    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                    rx.borrow_and_update();
                    let _ = handle.emit("jobs", jm.summaries());
                }
            });

            app.manage(Ctx { api, sources, store, jobs });
            #[cfg(desktop)]
            desktop::setup(app)?;

            // 页面就绪后再显示窗口，避免白屏闪烁
            if let Some(w) = app.get_webview_window("main") {
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_millis(120)).await;
                    let _ = w.show();
                });
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            home,
            sources,
            resolve,
            search,
            song,
            song_url,
            lyric,
            playlist,
            album,
            toplist,
            match_playable,
            set_source_cookie,
            account,
            set_cookie,
            logout,
            qr_start,
            qr_check,
            add_job,
            jobs,
            job,
            job_action,
            remove_job,
            clear_jobs,
            set_concurrency,
            default_dir_cmd,
            reveal,
            set_lyrics_window,
            set_lyrics_locked,
            set_mini_window,
            set_desktop_prefs
        ])
        .run(tauri::generate_context!())
        .expect("启动拾音失败");
}
