mod routes;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use shiyin_core::store::CredentialStore;
use shiyin_core::Netease;

pub struct AppState {
    pub api: Netease,
    pub store: CredentialStore,
    pub token: Option<String>,
    pub tmp: PathBuf,
}

fn data_dir() -> PathBuf {
    if let Ok(d) = std::env::var("SHIYIN_DATA_DIR") {
        return PathBuf::from(d);
    }
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
        .or_else(|| std::env::var_os("APPDATA").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("shiyin")
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_env("SHIYIN_LOG").unwrap_or_else(|_| "info".into()),
        )
        .init();

    let addr: SocketAddr = std::env::var("SHIYIN_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:6970".into())
        .parse()
        .expect("SHIYIN_ADDR 格式应为 IP:端口");
    let token = std::env::var("SHIYIN_TOKEN").ok().filter(|t| !t.is_empty());
    if token.is_none() && !addr.ip().is_loopback() {
        eprintln!("监听非本机地址时必须设置 SHIYIN_TOKEN");
        std::process::exit(2);
    }

    let dir = data_dir();
    let store = CredentialStore::new(&dir);
    let api = Netease::new();
    api.set_music_u(store.load().or_else(|| std::env::var("MUSIC_U").ok()));
    let tmp = dir.join("tmp");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).expect("无法创建临时目录");

    let state = Arc::new(AppState { api, store, token, tmp });
    let app = routes::router(state);

    let listener = tokio::net::TcpListener::bind(addr).await.expect("端口被占用");
    tracing::info!("拾音已启动：http://{addr}");
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await
        .unwrap();
}
