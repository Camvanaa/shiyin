//! 网易云 eapi 参数加密：AES-128-ECB + PKCS7，摘要为 MD5。

use aes::cipher::{block_padding::Pkcs7, BlockEncryptMut, KeyInit};
use md5::{Digest, Md5};

const EAPI_KEY: &[u8; 16] = b"e82ckenh8dichen8";
const SEP: &str = "-36cd479b6b5-";

type Aes128EcbEnc = ecb::Encryptor<aes::Aes128>;

fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0xf) as usize] as char);
    }
    s
}

/// `path` 为 `/api/...` 形式（不是 `/eapi/`），`body` 为 JSON 文本。返回十六进制密文。
pub fn eapi_params(path: &str, body: &str) -> String {
    let digest = hex(&Md5::digest(format!("nobody{path}use{body}md5forencrypt")));
    let plain = format!("{path}{SEP}{body}{SEP}{digest}");
    let enc = Aes128EcbEnc::new(EAPI_KEY.into()).encrypt_padded_vec_mut::<Pkcs7>(plain.as_bytes());
    hex(&enc)
}

#[cfg(test)]
mod tests {
    use super::*;
    const PY_REF: &str = "4dc723619a991588865191fd2f319bad03511c3ceea3ae811a83dbbeda6b06a059434be9dc7ff770766dbc80774265da63ccfa81c3fb32b994912efe8c0dd96acfacf9ab10775dc02ff95ab6b9415a3b";

    #[test]
    fn matches_python_reference() {
        // 期望值由上游 Python 实现对同一输入计算得到。
        assert_eq!(eapi_params("/api/test", "{\"a\": 1}"), PY_REF);
    }
}
