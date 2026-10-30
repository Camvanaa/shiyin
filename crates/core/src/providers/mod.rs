//! 各音乐平台的适配器实现。

pub mod kuwo;
pub mod qq;

use serde_json::Value;

pub(crate) fn s(v: &Value) -> String {
    v.as_str().map(str::to_owned).unwrap_or_else(|| match v {
        Value::Number(n) => n.to_string(),
        _ => String::new(),
    })
}

pub(crate) fn u(v: &Value) -> u64 {
    v.as_u64()
        .or_else(|| v.as_f64().map(|f| f as u64))
        .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
        .unwrap_or(0)
}

pub(crate) fn first_str<'a>(v: &'a Value, keys: &[&str]) -> &'a Value {
    for k in keys {
        let parts: Vec<&str> = k.split('.').collect();
        let mut cur = v;
        for p in &parts {
            cur = &cur[*p];
        }
        if !cur.is_null() {
            return cur;
        }
    }
    &Value::Null
}

/// 数字 ID 提取：从文本或链接里取出目标 ID。
pub(crate) fn extract_id(text: &str, query_keys: &[&str], path_marker: &str) -> Option<String> {
    let text = text.trim();
    // 链接形式
    if let Some(pos) = text.find(path_marker) {
        let rest = &text[pos + path_marker.len()..];
        let id: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
        if !id.is_empty() {
            return Some(id);
        }
    }
    // query 形式 id= / rid= / songmid=
    for key in query_keys {
        if let Some(pos) = text.find(&format!("{key}=")) {
            let rest = &text[pos + key.len() + 1..];
            let id: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
            if !id.is_empty() {
                return Some(id);
            }
        }
    }
    // 纯 ID
    let clean = text.trim_start_matches("MUSIC_");
    if !clean.is_empty() && clean.len() <= 32 && clean.chars().all(|c| c.is_alphanumeric()) {
        return Some(clean.to_owned());
    }
    None
}
