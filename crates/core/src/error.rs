use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("网络请求失败：{0}")]
    Http(#[from] reqwest::Error),
    #[error("接口返回错误（{code}）：{msg}")]
    Api { code: i64, msg: String },
    #[error("无法获取播放地址，可能需要登录或会员")]
    Unavailable,
    #[error("数据解析失败：{0}")]
    Parse(String),
    #[error("文件读写失败：{0}")]
    Io(#[from] std::io::Error),
    #[error("写入元数据失败：{0}")]
    Tag(String),
    #[error("已取消")]
    Cancelled,
    #[error("{0}")]
    Invalid(String),
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::Parse(e.to_string())
    }
}

impl Serialize for Error {
    fn serialize<S: Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;
