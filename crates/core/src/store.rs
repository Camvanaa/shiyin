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
