use crate::model::Resolved;

pub fn first_url(text: &str) -> Option<&str> {
    let start = text.find("http://").or_else(|| text.find("https://"))?;
    let rest = &text[start..];
    let end = rest.find(|c: char| c.is_whitespace() || "，。\"'<>".contains(c)).unwrap_or(rest.len());
    Some(&rest[..end])
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
    Ok(Resolved::Query(text.to_owned()))
}


