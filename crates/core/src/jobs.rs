//! 下载任务管理器。桌面版和 Web 版共用。
//!
//! 一个任务（Job）包含一首或多首歌。全局限制同时下载的歌曲数。
//! 暂停会中断进行中的歌曲并在继续时重新下载它们。

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use serde::Serialize;
use tokio::sync::{watch, Semaphore};

use crate::source::DynSource;
use crate::download::{self, Cancel, DownloadItem, DownloadOptions};
use crate::error::Error;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Waiting,
    Running,
    Paused,
    Done,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemState {
    pub id: u64,
    pub name: String,
    pub artist: String,
    pub status: Status,
    pub done: u64,
    pub total: u64,
    pub error: Option<String>,
    pub file: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobSnapshot {
    pub id: u64,
    pub title: String,
    pub subtitle: String,
    pub cover: String,
    pub status: Status,
    pub dir: PathBuf,
    pub items: Vec<ItemState>,
    pub completed: usize,
    pub failed: usize,
    /// 当前速度，字节/秒
    pub speed: u64,
    /// Web 版：任务完成后可下载的文件
    pub artifact: Option<PathBuf>,
    pub created: u64,
}

/// 推送给界面的精简状态，不含全部歌曲，避免大歌单时数据量过大。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobSummary {
    pub id: u64,
    pub title: String,
    pub subtitle: String,
    pub cover: String,
    pub status: Status,
    pub total: usize,
    pub completed: usize,
    pub failed: usize,
    pub speed: u64,
    /// 0..1
    pub progress: f64,
    pub running: Vec<ItemState>,
    pub has_artifact: bool,
    pub dir: PathBuf,
    pub created: u64,
}

impl JobSnapshot {
    pub fn summary(&self) -> JobSummary {
        let running: Vec<ItemState> = self.items.iter().filter(|i| i.status == Status::Running).cloned().collect();
        let partial: f64 = running.iter().map(|i| if i.total > 0 { i.done as f64 / i.total as f64 } else { 0.0 }).sum();
        let total = self.items.len().max(1);
        JobSummary {
            id: self.id,
            title: self.title.clone(),
            subtitle: self.subtitle.clone(),
            cover: self.cover.clone(),
            status: self.status,
            total: self.items.len(),
            completed: self.completed,
            failed: self.failed,
            speed: self.speed,
            progress: ((self.completed + self.failed) as f64 + partial) / total as f64,
            running,
            has_artifact: self.artifact.is_some(),
            dir: self.dir.clone(),
            created: self.created,
        }
    }
}

struct Job {
    snap: JobSnapshot,
    items: Vec<DownloadItem>,
    opts: DownloadOptions,
    cancels: HashMap<usize, Cancel>,
    speed_window: (Instant, u64),
    workers: usize,
    finished_at: Option<Instant>,
    /// 已被 worker 领取但还没开始下载的歌曲
    claimed: std::collections::HashSet<usize>,
}

/// 任务完成后的处理（Web 版用来打包 ZIP）。返回产物路径。
pub type Finisher = Arc<dyn Fn(&JobSnapshot) -> Option<PathBuf> + Send + Sync>;

#[derive(Clone)]
pub struct JobManager {
    inner: Arc<Inner>,
}

struct Inner {
    src: DynSource,
    jobs: Mutex<Vec<Job>>,
    slots: Arc<Semaphore>,
    limit: Mutex<usize>,
    version: watch::Sender<u64>,
    next_id: Mutex<u64>,
    finisher: Option<Finisher>,
}

fn now_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

pub struct NewJob {
    pub title: String,
    pub subtitle: String,
    pub cover: String,
    pub items: Vec<DownloadItem>,
    pub opts: DownloadOptions,
    pub dir: PathBuf,
}

impl JobManager {
    pub fn new(src: DynSource, concurrency: usize, finisher: Option<Finisher>) -> Self {
        let c = concurrency.clamp(1, 10);
        Self {
            inner: Arc::new(Inner {
                src,
                jobs: Mutex::new(Vec::new()),
                slots: Arc::new(Semaphore::new(c)),
                limit: Mutex::new(c),
                version: watch::channel(0).0,
                next_id: Mutex::new(1),
                finisher,
            }),
        }
    }

    /// 订阅变化通知（值为单调递增的版本号）。
    pub fn subscribe(&self) -> watch::Receiver<u64> {
        self.inner.version.subscribe()
    }

    fn bump(&self) {
        self.inner.version.send_modify(|v| *v += 1);
    }

    pub fn set_concurrency(&self, n: usize) {
        let n = n.clamp(1, 10);
        let mut limit = self.inner.limit.lock().unwrap();
        if n > *limit {
            self.inner.slots.add_permits(n - *limit);
        } else if n < *limit {
            let slots = self.inner.slots.clone();
            let diff = (*limit - n) as u32;
            tokio::spawn(async move {
                if let Ok(p) = slots.acquire_many(diff).await {
                    p.forget();
                }
            });
        }
        *limit = n;
    }

    pub fn summaries(&self) -> Vec<JobSummary> {
        self.inner.jobs.lock().unwrap().iter().map(|j| j.snap.summary()).collect()
    }

    pub fn get(&self, id: u64) -> Option<JobSnapshot> {
        self.inner.jobs.lock().unwrap().iter().find(|j| j.snap.id == id).map(|j| j.snap.clone())
    }

    pub fn add(&self, job: NewJob) -> u64 {
        let id = {
            let mut n = self.inner.next_id.lock().unwrap();
            *n += 1;
            *n
        };
        let items = job
            .items
            .iter()
            .map(|it| ItemState {
                id: it.track.id,
                name: it.track.name.clone(),
                artist: it.track.artist_names(),
                status: Status::Waiting,
                done: 0,
                total: 0,
                error: None,
                file: None,
            })
            .collect();
        let snap = JobSnapshot {
            id,
            title: job.title,
            subtitle: job.subtitle,
            cover: job.cover,
            status: Status::Waiting,
            dir: job.dir,
            items,
            completed: 0,
            failed: 0,
            speed: 0,
            artifact: None,
            created: now_ms(),
        };
        self.inner.jobs.lock().unwrap().push(Job {
            snap,
            items: job.items,
            opts: job.opts,
            cancels: HashMap::new(),
            speed_window: (Instant::now(), 0),
            workers: 0,
            finished_at: None,
            claimed: Default::default(),
        });
        self.bump();
        self.spawn_items(id);
        id
    }

    /// 每个任务最多启动 10 个 worker，worker 依次领取等待中的歌曲，
    /// 全局并发由信号量控制。大歌单不会一次创建上千个异步任务。
    fn spawn_items(&self, id: u64) {
        let workers = self
            .with_job(id, |job| {
                let waiting = job.snap.items.iter().filter(|i| i.status == Status::Waiting).count();
                let n = waiting.min(10).saturating_sub(job.workers);
                job.workers += n;
                n
            })
            .unwrap_or(0);
        for _ in 0..workers {
            let me = self.clone();
            tokio::spawn(async move {
                me.worker(id).await;
                me.with_job(id, |job| job.workers -= 1);
            });
        }
    }

    async fn worker(&self, id: u64) {
        loop {
            let next = self.with_job(id, |job| {
                if job.snap.status == Status::Paused || job.snap.status == Status::Cancelled {
                    return None;
                }
                let claimed = &job.claimed;
                let idx = (0..job.snap.items.len())
                    .find(|i| job.snap.items[*i].status == Status::Waiting && !claimed.contains(i))?;
                job.claimed.insert(idx);
                Some(idx)
            });
            let Some(Some(idx)) = next else { return };
            self.run_item(id, idx).await;
            self.with_job(id, |job| job.claimed.remove(&idx));
        }
    }

    fn with_job<R>(&self, id: u64, f: impl FnOnce(&mut Job) -> R) -> Option<R> {
        let mut jobs = self.inner.jobs.lock().unwrap();
        jobs.iter_mut().find(|j| j.snap.id == id).map(f)
    }

    async fn run_item(&self, id: u64, idx: usize) {
        loop {
            let Ok(permit) = self.inner.slots.clone().acquire_owned().await else { return };
            let cancel = Cancel::default();
            let prepared = self.with_job(id, |job| {
                if job.snap.items[idx].status != Status::Waiting || job.snap.status == Status::Paused {
                    return None;
                }
                job.snap.items[idx].status = Status::Running;
                job.snap.items[idx].done = 0;
                job.snap.status = Status::Running;
                job.cancels.insert(idx, cancel.clone());
                Some((job.items[idx].clone(), job.opts.clone(), job.snap.dir.clone()))
            });
            let Some(Some((item, opts, dir))) = prepared else { return };
            self.bump();

            let mut last = Instant::now();
            let me = self.clone();
            let res = download::download(self.inner.src.as_ref(), &item, &opts, &dir, &cancel, move |done, total| {
                // 限制更新频率，避免锁竞争
                if last.elapsed().as_millis() < 120 && done < total {
                    return;
                }
                last = Instant::now();
                me.with_job(id, |job| {
                    let prev = job.snap.items[idx].done;
                    job.snap.items[idx].done = done;
                    job.snap.items[idx].total = total;
                    job.speed_window.1 += done.saturating_sub(prev);
                    let el = job.speed_window.0.elapsed().as_secs_f64();
                    if el >= 0.5 {
                        job.snap.speed = (job.speed_window.1 as f64 / el) as u64;
                        job.speed_window = (Instant::now(), 0);
                    }
                });
                me.bump();
            })
            .await;
            drop(permit);

            let mut requeue = false;
            let finished = self.with_job(id, |job| {
                job.cancels.remove(&idx);
                let st = &mut job.snap.items[idx];
                match res {
                    Ok(d) => {
                        st.status = Status::Done;
                        st.done = d.size;
                        st.total = d.size;
                        st.file = Some(d.path);
                    }
                    Err(Error::Cancelled) => {
                        // 暂停时回到等待；暂停后又很快继续时需要重新排队。
                        if job.snap.status == Status::Cancelled {
                            st.status = Status::Cancelled;
                        } else {
                            st.status = Status::Waiting;
                            st.done = 0;
                            if job.snap.status != Status::Paused {
                                requeue = true;
                            }
                        }
                    }
                    Err(e) => {
                        st.status = Status::Failed;
                        st.error = Some(e.to_string());
                    }
                }
                job.snap.completed = job.snap.items.iter().filter(|i| i.status == Status::Done).count();
                job.snap.failed = job.snap.items.iter().filter(|i| i.status == Status::Failed).count();
                let pending = job.snap.items.iter().any(|i| matches!(i.status, Status::Waiting | Status::Running));
                if !pending && !matches!(job.snap.status, Status::Paused | Status::Cancelled) {
                    job.snap.status = if job.snap.completed > 0 { Status::Done } else { Status::Failed };
                    job.snap.speed = 0;
                    job.finished_at = Some(Instant::now());
                    return Some(job.snap.clone());
                }
                if !job.snap.items.iter().any(|i| i.status == Status::Running) {
                    job.snap.speed = 0;
                }
                None
            });
            self.bump();
            if requeue {
                continue;
            }

            if let (Some(Some(snap)), Some(f)) = (finished, self.inner.finisher.clone()) {
                let artifact = tokio::task::spawn_blocking(move || f(&snap)).await.ok().flatten();
                self.with_job(id, |job| job.snap.artifact = artifact);
                self.bump();
            }
            break;
        }
    }

    pub fn pause(&self, id: u64) {
        self.with_job(id, |job| {
            if matches!(job.snap.status, Status::Waiting | Status::Running) {
                job.snap.status = Status::Paused;
                job.snap.speed = 0;
                for c in job.cancels.values() {
                    c.cancel();
                }
            }
        });
        self.bump();
    }

    pub fn resume(&self, id: u64) {
        let ok = self
            .with_job(id, |job| {
                if job.snap.status != Status::Paused {
                    return false;
                }
                job.snap.status = Status::Waiting;
                true
            })
            .unwrap_or(false);
        if ok {
            self.bump();
            self.spawn_items(id);
        }
    }

    pub fn cancel(&self, id: u64) {
        self.with_job(id, |job| {
            if matches!(job.snap.status, Status::Done | Status::Failed | Status::Cancelled) {
                return;
            }
            job.snap.status = Status::Cancelled;
            job.finished_at = Some(Instant::now());
            job.snap.speed = 0;
            for c in job.cancels.values() {
                c.cancel();
            }
            for it in &mut job.snap.items {
                if it.status == Status::Waiting {
                    it.status = Status::Cancelled;
                }
            }
        });
        self.bump();
    }

    /// 重试失败的歌曲
    pub fn retry(&self, id: u64) {
        let ok = self
            .with_job(id, |job| {
                let mut any = false;
                for it in &mut job.snap.items {
                    if matches!(it.status, Status::Failed | Status::Cancelled) {
                        it.status = Status::Waiting;
                        it.error = None;
                        it.done = 0;
                        any = true;
                    }
                }
                if any {
                    job.snap.status = Status::Waiting;
                    job.snap.artifact = None;
                    job.finished_at = None;
                    job.snap.failed = 0;
                }
                any
            })
            .unwrap_or(false);
        if ok {
            self.bump();
            self.spawn_items(id);
        }
    }

    /// 从列表移除已结束的任务，返回被移除的任务。
    pub fn remove(&self, id: u64) -> Option<JobSnapshot> {
        self.cancel(id);
        let mut jobs = self.inner.jobs.lock().unwrap();
        let pos = jobs.iter().position(|j| j.snap.id == id)?;
        let j = jobs.remove(pos);
        drop(jobs);
        self.bump();
        Some(j.snap)
    }

    /// 移除结束时间早于 `age` 之前的任务
    pub fn clear_finished_before(&self, age: std::time::Duration) -> Vec<JobSnapshot> {
        let now = Instant::now();
        let mut jobs = self.inner.jobs.lock().unwrap();
        let mut removed = Vec::new();
        jobs.retain(|j| {
            let old = j.finished_at.is_some_and(|t| now.duration_since(t) >= age);
            if old {
                removed.push(j.snap.clone());
            }
            !old
        });
        drop(jobs);
        if !removed.is_empty() {
            self.bump();
        }
        removed
    }

    pub fn clear_finished(&self) -> Vec<JobSnapshot> {
        let mut jobs = self.inner.jobs.lock().unwrap();
        let mut removed = Vec::new();
        jobs.retain(|j| {
            let fin = matches!(j.snap.status, Status::Done | Status::Failed | Status::Cancelled);
            if fin {
                removed.push(j.snap.clone());
            }
            !fin
        });
        drop(jobs);
        self.bump();
        removed
    }
}
