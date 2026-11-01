//! 凭证持久化。文件只保存 MUSIC_U，Unix 下权限为 0600。

use std::path::{Path, PathBuf};

use crate::error::Result;

pub struct CredentialStore {
    path: PathBuf,
}

impl CredentialStore {
    pub fn new(dir: &Path) -> Self {
        Self { path: dir.join("credential") }
    }

    pub fn load(&self) -> Option<String> {
        std::fs::read_to_string(&self.path).ok().map(|s| s.trim().to_owned()).filter(|s| !s.is_empty())
    }

    pub fn save(&self, music_u: Option<&str>) -> Result<()> {
        match music_u {
            None => match std::fs::remove_file(&self.path) {
                Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.into()),
                _ => Ok(()),
            },
            Some(v) => {
                if let Some(dir) = self.path.parent() {
                    std::fs::create_dir_all(dir)?;
                }
                let tmp = self.path.with_extension("tmp");
                {
                    let mut opts = std::fs::OpenOptions::new();
                    opts.write(true).create(true).truncate(true);
                    #[cfg(unix)]
                    std::os::unix::fs::OpenOptionsExt::mode(&mut opts, 0o600);
                    use std::io::Write;
                    opts.open(&tmp)?.write_all(v.as_bytes())?;
                }
                std::fs::rename(&tmp, &self.path)?;
                Ok(())
            }
        }
    }
}

impl CredentialStore {
    fn key_path(&self, source: &str) -> PathBuf {
        let safe: String = source.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
        self.path.with_file_name(format!("credential.{safe}"))
    }

    pub fn load_for(&self, source: &str) -> Option<String> {
        if source == "netease" {
            if let Some(v) = self.load() {
                return Some(v);
            }
        }
        std::fs::read_to_string(self.key_path(source))
            .ok()
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty())
    }

    pub fn save_for(&self, source: &str, cookie: Option<&str>) -> Result<()> {
        if source == "netease" {
            return self.save(cookie);
        }
        let p = self.key_path(source);
        match cookie.map(str::trim).filter(|s| !s.is_empty()) {
            None => match std::fs::remove_file(&p) {
                Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.into()),
                _ => Ok(()),
            },
            Some(v) => {
                if let Some(dir) = p.parent() {
                    std::fs::create_dir_all(dir)?;
                }
                let mut opts = std::fs::OpenOptions::new();
                opts.write(true).create(true).truncate(true);
                #[cfg(unix)]
                std::os::unix::fs::OpenOptionsExt::mode(&mut opts, 0o600);
                use std::io::Write;
                opts.open(&p)?.write_all(v.as_bytes())?;
                Ok(())
            }
        }
    }
}

/// 音源选项持久化：一个 JSON 文件 `{ "apple": { "storefront": "hk" } }`，不含凭证。
impl CredentialStore {
    fn options_path(&self) -> PathBuf {
        self.path.with_file_name("source-options.json")
    }

    pub fn load_option(&self, source: &str, key: &str) -> Option<String> {
        let text = std::fs::read_to_string(self.options_path()).ok()?;
        let v: serde_json::Value = serde_json::from_str(&text).ok()?;
        v[source][key].as_str().map(str::to_owned)
    }

    pub fn save_option(&self, source: &str, key: &str, value: &str) -> Result<()> {
        let p = self.options_path();
        let mut v: serde_json::Value = std::fs::read_to_string(&p)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_else(|| serde_json::json!({}));
        v[source][key] = serde_json::Value::String(value.to_owned());
        if let Some(dir) = p.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(&p, serde_json::to_vec_pretty(&v)?)?;
        Ok(())
    }
}
