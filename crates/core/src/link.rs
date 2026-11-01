use crate::model::Resolved;

pub fn first_url(text: &str) -> Option<&str> {
    let start = text.find("http://").or_else(|| text.find("https://"))?;
    let rest = &text[start..];
    let end = rest.find(|c: char| c.is_whitespace() || "，。\"'<>".contains(c)).unwrap_or(rest.len());
    Some(&rest[..end])
}

/// 是否为网易云官方短链（服务端会替用户请求该地址，必须校验主机名）
pub fn is_163_short_link(url: &str) -> bool {
    reqwest::Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(|h| h.to_ascii_lowercase()))
        .is_some_and(|h| h == "163cn.tv" || h == "y.music.163.com" || h.ends_with(".163cn.tv"))
}

fn query_id(url: &str) -> Option<u64> {
    let q = url.split_once("id=")?.1;
    let digits: String = q.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().ok()
}

/// 不访问网络的识别部分。短链接返回 `Err(url)`，由调用方展开后再识别。
pub fn classify(text: &str) -> std::result::Result<Resolved, String> {
    let text = text.trim();
    if let Some(url) = first_url(text) {
        if url.contains("163cn.tv") {
            return Err(url.to_owned());
        }
        if url.contains("music.163.com") || url.contains("y.music.163.com") {
            // 形如 /song?id=、/#/playlist?id=、/m/album?id=、/song/123
            let lower = url.to_ascii_lowercase();
            let id = query_id(url).or_else(|| {
                lower
                    .rsplit('/')
                    .find_map(|seg| seg.split(['?', '&']).next().and_then(|s| s.parse().ok()))
            });
            if let Some(id) = id {
                let id = id.to_string();
                let source = "netease".to_string();
                return Ok(if lower.contains("playlist") {
                    Resolved::Playlist { source, id }
                } else if lower.contains("album") {
                    Resolved::Album { source, id }
                } else {
                    Resolved::Song { source, id }
                });
            }
        }
    }
    if !text.is_empty() && text.len() <= 20 && text.chars().all(|c| c.is_ascii_digit()) {
        if let Ok(id) = text.parse::<u64>() {
            return Ok(Resolved::Song { source: "netease".into(), id: id.to_string() });
        }
    }
    Ok(Resolved::Query { value: text.to_owned() })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_163_hosts_are_expandable() {
        assert!(is_163_short_link("https://163cn.tv/abc123"));
        assert!(is_163_short_link("http://y.music.163.com/m/song/1"));
        assert!(is_163_short_link("https://x.163cn.tv/abc"));
        // 不能因为文本里出现 "163cn.tv" 就替用户请求任意地址（SSRF）
        assert!(!is_163_short_link("http://169.254.169.254/latest/meta-data"));
        assert!(!is_163_short_link("http://127.0.0.1:6970/api/sources"));
        assert!(!is_163_short_link("https://evil.com/163cn.tv"));
        assert!(!is_163_short_link("不是链接"));
    }

    #[test]
    fn first_url_stops_at_whitespace_and_punctuation() {
        assert_eq!(first_url("看看 https://163cn.tv/abc，好听"), Some("https://163cn.tv/abc"));
        assert_eq!(first_url("没有链接"), None);
    }
}


