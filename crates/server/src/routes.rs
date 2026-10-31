use std::convert::Infallible;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::extract::{Path as UrlPath, Query, Request, State};
use axum::http::{header, HeaderValue, StatusCode, Uri};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use shiyin_core::download::{DownloadItem, DownloadOptions};
use shiyin_core::jobs::{JobManager, NewJob, Status};
use shiyin_core::{Error as CoreError, Track};
use subtle::ConstantTimeEq;

use crate::AppState;

#[derive(rust_embed::Embed)]
#[folder = "../../web/dist"]
struct Assets;

pub struct ApiError(StatusCode, String);

impl From<CoreError> for ApiError {
    fn from(e: CoreError) -> Self {
        let code = match e {
            CoreError::Invalid(_) => StatusCode::BAD_REQUEST,
            CoreError::Unavailable => StatusCode::FORBIDDEN,
            _ => StatusCode::BAD_GATEWAY,
        };
        ApiError(code, e.to_string())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(json!({ "error": self.1 }))).into_response()
    }
}

type R<T> = Result<Json<T>, ApiError>;

#[derive(Clone)]
struct Ctx {
    app: Arc<AppState>,
    jobs: JobManager,
}

pub fn router(app: Arc<AppState>) -> Router {
    let finisher: shiyin_core::jobs::Finisher = Arc::new(|snap| {
        let files: Vec<PathBuf> = snap.items.iter().filter_map(|i| i.file.clone()).collect();
        if files.len() == 1 && snap.items.len() == 1 && !files[0].with_extension("lrc").exists() {
            return files.into_iter().next();
        }
        if files.is_empty() {
            return None;
        }
        zip_files(&snap.dir, &snap.title, &files).ok()
    });
    let jobs = JobManager::new(app.sources.clone(), 3, Some(finisher));

    // 定期清理结束超过 2 小时的任务及其临时文件
    let jm = jobs.clone();
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(600));
        loop {
            tick.tick().await;
            for s in jm.clear_finished_before(Duration::from_secs(7200)) {
                let _ = tokio::fs::remove_dir_all(&s.dir).await;
            }
        }
    });

    let ctx = Ctx { app: app.clone(), jobs };

    let api = Router::new()
        .route("/sources", get(sources))
        .route("/home", get(home))
        .route("/resolve", get(resolve))
        .route("/search", get(search))
        .route("/song/{id}", get(song))
        .route("/url/{id}", get(song_url))
        .route("/lyric/{id}", get(lyric))
        .route("/playlist/{id}", get(playlist))
        .route("/album/{id}", get(album))
        .route("/toplist/{id}", get(toplist))
        .route("/match", get(match_playable))
        .route("/account", get(account).delete(logout))
        .route("/account/cookie", post(set_cookie))
        .route("/sources/{id}/cookie", post(set_source_cookie).delete(clear_source_cookie))
        .route("/qr", post(qr_start))
        .route("/qr/{key}", get(qr_check))
        .route("/jobs", get(list_jobs).post(add_job).delete(clear_jobs))
        .route("/jobs/events", get(job_events))
        .route("/jobs/concurrency", post(set_concurrency))
        .route("/jobs/{id}", get(get_job).delete(remove_job))
        .route("/jobs/{id}/{action}", post(job_action))
        .route("/jobs/{id}/file", get(job_file))
        .layer(middleware::from_fn_with_state(app, auth))
        .with_state(ctx);

    Router::new()
        .nest("/api", api)
        .fallback(static_file)
        .layer(tower_http::compression::CompressionLayer::new().gzip(true).br(true))
}

async fn auth(State(app): State<Arc<AppState>>, req: Request, next: Next) -> Response {
    if let Some(token) = &app.token {
        let given = req
            .headers()
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .map(str::to_owned)
            .or_else(|| {
                // EventSource 和下载链接无法设置请求头，允许用查询参数传递
                req.uri().query().and_then(|q| {
                    q.split('&').find_map(|kv| kv.strip_prefix("token=").map(str::to_owned))
                })
            })
            .unwrap_or_default();
        if !bool::from(given.as_bytes().ct_eq(token.as_bytes())) {
            return ApiError(StatusCode::UNAUTHORIZED, "需要访问口令".into()).into_response();
        }
    }
    next.run(req).await
}

async fn static_file(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let (file, path) = match Assets::get(path) {
        Some(f) if !path.is_empty() => (f, path),
        _ => (Assets::get("index.html").expect("index.html"), "index.html"),
    };
    let mime = mime_guess::from_path(path).first_or_octet_stream();
    let cache = if path.starts_with("assets/") { "public, max-age=31536000, immutable" } else { "no-cache" };
    (
        [(header::CONTENT_TYPE, mime.as_ref().to_owned()), (header::CACHE_CONTROL, cache.to_owned())],
        file.data,
    )
        .into_response()
}

#[derive(Deserialize)]
struct Q {
    q: String,
}

async fn home(State(c): State<Ctx>, Query(q): Query<SourceQ>) -> R<shiyin_core::HomeFeed> {
    Ok(Json(c.app.sources.home(&q.source).await?))
}

async fn resolve(State(c): State<Ctx>, Query(q): Query<Q>) -> R<Value> {
    Ok(Json(serde_json::to_value(c.app.sources.resolve(&q.q).await?).unwrap()))
}

async fn sources(State(c): State<Ctx>) -> Json<Value> {
    let list: Vec<Value> = c
        .app
        .sources
        .list()
        .iter()
        .map(|s| json!({ "id": s.id(), "label": s.label(), "levels": s.levels(), "hasCookie": s.has_cookie() }))
        .collect();
    Json(json!(list))
}

#[derive(Deserialize)]
struct SearchQ {
    q: String,
    #[serde(default = "default_limit")]
    limit: u32,
    #[serde(default)]
    offset: u32,
    #[serde(default)]
    source: String,
}

fn default_limit() -> u32 {
    30
}

async fn search(State(c): State<Ctx>, Query(q): Query<SearchQ>) -> R<Vec<Track>> {
    if q.source.is_empty() || q.source == "all" {
        return Ok(Json(c.app.sources.search_all(&q.q, q.limit, q.offset).await));
    }
    let src = pick(&c, &q.source)?;
    Ok(Json(src.search(&q.q, q.limit, q.offset).await?))
}

async fn song(State(c): State<Ctx>, UrlPath(id): UrlPath<String>, Query(q): Query<SourceQ>) -> R<Track> {
    Ok(Json(pick(&c, &q.source)?.track(&id).await?))
}

#[derive(Deserialize)]
struct LevelQ {
    level: String,
    #[serde(default)]
    source: String,
}

#[derive(Deserialize)]
struct SourceQ {
    #[serde(default)]
    source: String,
}

fn pick<'a>(c: &'a Ctx, source: &str) -> Result<&'a shiyin_core::DynSource, ApiError> {
    c.app.sources.pick(source).map_err(ApiError::from)
}

async fn song_url(State(c): State<Ctx>, UrlPath(id): UrlPath<String>, Query(q): Query<LevelQ>) -> R<shiyin_core::SongUrl> {
    Ok(Json(pick(&c, &q.source)?.song_url(&id, &q.level).await?))
}

async fn lyric(State(c): State<Ctx>, UrlPath(id): UrlPath<String>, Query(q): Query<SourceQ>) -> R<shiyin_core::Lyric> {
    Ok(Json(pick(&c, &q.source)?.lyric(&id).await?))
}

async fn playlist(State(c): State<Ctx>, UrlPath(id): UrlPath<String>, Query(q): Query<SourceQ>) -> R<shiyin_core::Collection> {
    Ok(Json(pick(&c, &q.source)?.collection(shiyin_core::CollectionKind::Playlist, &id).await?))
}

async fn album(State(c): State<Ctx>, UrlPath(id): UrlPath<String>, Query(q): Query<SourceQ>) -> R<shiyin_core::Collection> {
    Ok(Json(pick(&c, &q.source)?.collection(shiyin_core::CollectionKind::Album, &id).await?))
}

async fn toplist(State(c): State<Ctx>, UrlPath(id): UrlPath<String>, Query(q): Query<SourceQ>) -> R<shiyin_core::Collection> {
    Ok(Json(pick(&c, &q.source)?.collection(shiyin_core::CollectionKind::Toplist, &id).await?))
}

#[derive(Deserialize)]
struct MatchQ {
    name: String,
    #[serde(default)]
    artist: String,
    #[serde(default)]
    skip: String,
    #[serde(default = "default_level")]
    level: String,
}
fn default_level() -> String { "exhigh".into() }

async fn match_playable(State(c): State<Ctx>, Query(q): Query<MatchQ>) -> R<Value> {
    let Some((track, url)) = c.app.sources.find_playable(&q.name, &q.artist, &q.skip, &q.level).await else {
        return Err(ApiError(StatusCode::NOT_FOUND, "其它音源也未找到可播放版本".into()));
    };
    Ok(Json(json!({ "track": track, "url": url })))
}

#[derive(Deserialize)]
struct SourceCookieBody {
    cookie: String,
}

async fn set_source_cookie(State(c): State<Ctx>, UrlPath(id): UrlPath<String>, Json(b): Json<SourceCookieBody>) -> R<Value> {
    let src = pick(&c, &id)?;
    src.set_cookie(Some(b.cookie.clone()));
    c.app.store.save_for(&id, Some(&b.cookie))?;
    Ok(Json(json!({ "ok": true, "hasCookie": src.has_cookie() })))
}

async fn clear_source_cookie(State(c): State<Ctx>, UrlPath(id): UrlPath<String>) -> R<Value> {
    let src = pick(&c, &id)?;
    src.set_cookie(None);
    c.app.store.save_for(&id, None)?;
    Ok(Json(json!({ "ok": true, "hasCookie": false })))
}

async fn account(State(c): State<Ctx>) -> R<Value> {
    Ok(Json(json!({ "account": c.app.api.account().await?, "hasCredential": c.app.api.music_u().is_some() })))
}

async fn logout(State(c): State<Ctx>) -> R<Value> {
    c.app.api.set_music_u(None);
    c.app.store.save(None)?;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
struct CookieBody {
    #[serde(rename = "musicU")]
    music_u: String,
}

async fn set_cookie(State(c): State<Ctx>, Json(b): Json<CookieBody>) -> R<Value> {
    let prev = c.app.api.music_u();
    c.app.api.set_music_u(Some(b.music_u));
    match c.app.api.account().await {
        Ok(Some(acc)) => {
            c.app.store.save(c.app.api.music_u().as_deref())?;
            Ok(Json(json!({ "account": acc })))
        }
        other => {
            c.app.api.set_music_u(prev);
            let msg = match other {
                Err(e) => e.to_string(),
                _ => "MUSIC_U 无效或已过期".into(),
            };
            Err(ApiError(StatusCode::BAD_REQUEST, msg))
        }
    }
}

async fn qr_start(State(c): State<Ctx>) -> R<shiyin_core::QrStart> {
    Ok(Json(c.app.api.qr_start().await?))
}

async fn qr_check(State(c): State<Ctx>, UrlPath(key): UrlPath<String>) -> R<Value> {
    let (state, music_u) = c.app.api.qr_check(&key).await?;
    let mut account = None;
    if let Some(mu) = music_u {
        c.app.api.set_music_u(Some(mu));
        c.app.store.save(c.app.api.music_u().as_deref())?;
        account = c.app.api.account().await.ok().flatten();
    }
    Ok(Json(json!({ "state": state, "account": account })))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct JobBody {
    title: String,
    #[serde(default)]
    subtitle: String,
    #[serde(default)]
    cover: String,
    items: Vec<DownloadItem>,
    #[serde(default)]
    options: DownloadOptions,
}

async fn add_job(State(c): State<Ctx>, Json(b): Json<JobBody>) -> R<Value> {
    if b.items.is_empty() || b.items.len() > 1000 {
        return Err(ApiError(StatusCode::BAD_REQUEST, "单次最多下载 1000 首".into()));
    }
    if c.jobs.summaries().len() >= 20 {
        return Err(ApiError(StatusCode::TOO_MANY_REQUESTS, "任务过多，请先清除已完成的任务".into()));
    }
    if !shiyin_core::is_valid_level(&b.options.level) {
        return Err(ApiError(StatusCode::BAD_REQUEST, "未知音质".into()));
    }
    let dir = c.app.tmp.join(format!("job-{}", rand::random::<u64>()));
    let id = c.jobs.add(NewJob { title: b.title, subtitle: b.subtitle, cover: b.cover, items: b.items, opts: b.options, dir });
    Ok(Json(json!({ "id": id })))
}

async fn list_jobs(State(c): State<Ctx>) -> Json<Value> {
    Json(json!(c.jobs.summaries()))
}

async fn get_job(State(c): State<Ctx>, UrlPath(id): UrlPath<u64>) -> R<Value> {
    c.jobs.get(id).map(|j| Json(json!(j))).ok_or_else(|| ApiError(StatusCode::NOT_FOUND, "任务不存在".into()))
}

/// SSE：任务状态变化时推送全部任务摘要，最多每 250ms 一次。
async fn job_events(State(c): State<Ctx>) -> Response {
    let rx = c.jobs.subscribe();
    let jobs = c.jobs.clone();
    let stream = futures_util::stream::unfold((rx, jobs, true), |(mut rx, jobs, first)| async move {
        if !first {
            rx.changed().await.ok()?;
            tokio::time::sleep(Duration::from_millis(250)).await;
            rx.borrow_and_update();
        }
        let data = serde_json::to_string(&jobs.summaries()).ok()?;
        Some((Ok::<_, Infallible>(format!("data: {data}\n\n")), (rx, jobs, false)))
    });
    let keepalive = futures_util::stream::unfold((), |_| async {
        tokio::time::sleep(Duration::from_secs(20)).await;
        Some((Ok::<_, Infallible>(": ping\n\n".to_string()), ()))
    });
    let merged = futures_util::stream::select(stream, keepalive);
    Response::builder()
        .header(header::CONTENT_TYPE, "text/event-stream")
        .header(header::CACHE_CONTROL, "no-cache")
        .header("X-Accel-Buffering", "no")
        .body(Body::from_stream(merged))
        .unwrap()
}

#[derive(Deserialize)]
struct ConcBody {
    value: usize,
}

async fn set_concurrency(State(c): State<Ctx>, Json(b): Json<ConcBody>) -> Json<Value> {
    c.jobs.set_concurrency(b.value);
    Json(json!({ "ok": true }))
}

async fn job_action(State(c): State<Ctx>, UrlPath((id, action)): UrlPath<(u64, String)>) -> R<Value> {
    match action.as_str() {
        "pause" => c.jobs.pause(id),
        "resume" => c.jobs.resume(id),
        "cancel" => c.jobs.cancel(id),
        "retry" => c.jobs.retry(id),
        _ => return Err(ApiError(StatusCode::NOT_FOUND, "未知操作".into())),
    }
    Ok(Json(json!({ "ok": true })))
}

fn cleanup(snap: &shiyin_core::jobs::JobSnapshot) {
    let dir = snap.dir.clone();
    tokio::spawn(async move {
        // 留一点时间让进行中的写入退出
        tokio::time::sleep(Duration::from_secs(2)).await;
        let _ = tokio::fs::remove_dir_all(dir).await;
    });
}

async fn remove_job(State(c): State<Ctx>, UrlPath(id): UrlPath<u64>) -> Json<Value> {
    if let Some(s) = c.jobs.remove(id) {
        cleanup(&s);
    }
    Json(json!({ "ok": true }))
}

async fn clear_jobs(State(c): State<Ctx>) -> Json<Value> {
    for s in c.jobs.clear_finished() {
        cleanup(&s);
    }
    Json(json!({ "ok": true }))
}

async fn job_file(State(c): State<Ctx>, UrlPath(id): UrlPath<u64>) -> Result<Response, ApiError> {
    let job = c.jobs.get(id).ok_or_else(|| ApiError(StatusCode::NOT_FOUND, "任务不存在".into()))?;
    if job.status != Status::Done {
        return Err(ApiError(StatusCode::CONFLICT, "任务未完成".into()));
    }
    let path = job.artifact.ok_or_else(|| ApiError(StatusCode::NOT_FOUND, "没有可下载的文件".into()))?;
    let file = tokio::fs::File::open(&path).await.map_err(CoreError::from)?;
    let len = file.metadata().await.map_err(CoreError::from)?.len();
    let name = path.file_name().unwrap().to_string_lossy().into_owned();
    let mime = mime_guess::from_path(&path).first_or_octet_stream();
    let disp = format!("attachment; filename=\"download\"; filename*=UTF-8''{}", pct(&name));
    Ok(Response::builder()
        .header(header::CONTENT_TYPE, mime.as_ref())
        .header(header::CONTENT_LENGTH, len)
        .header(header::CONTENT_DISPOSITION, HeaderValue::from_str(&disp).unwrap())
        .body(Body::from_stream(tokio_util::io::ReaderStream::new(file)))
        .unwrap())
}

fn pct(s: &str) -> String {
    let mut o = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
            o.push(b as char);
        } else {
            o.push_str(&format!("%{b:02X}"));
        }
    }
    o
}

/// 打包为不压缩的 ZIP（音频本身已压缩，存储模式最快）。
fn zip_files(dir: &Path, title: &str, files: &[PathBuf]) -> std::io::Result<PathBuf> {
    use std::io::Write;
    let path = dir.join(format!("{}.zip", shiyin_core::download::sanitize(title)));
    let f = std::fs::File::create(&path)?;
    let mut z = zip::ZipWriter::new(std::io::BufWriter::new(f));
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Stored)
        .large_file(true);
    for p in files {
        let name = p.file_name().unwrap().to_string_lossy();
        z.start_file(name.as_ref(), opts)?;
        let mut src = std::fs::File::open(p)?;
        std::io::copy(&mut src, &mut z)?;
        let lrc = p.with_extension("lrc");
        if lrc.exists() {
            z.start_file(lrc.file_name().unwrap().to_string_lossy().as_ref(), opts)?;
            z.write_all(&std::fs::read(&lrc)?)?;
        }
    }
    z.finish()?.flush()?;
    // 打包后删除原文件以节省空间
    for p in files {
        let _ = std::fs::remove_file(p);
        let _ = std::fs::remove_file(p.with_extension("lrc"));
    }
    Ok(path)
}
