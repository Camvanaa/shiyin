use crate::api::Netease;
use crate::error::Result;
use crate::model::Resolved;

fn first_url(text: &str) -> Option<&str> {
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
                return Ok(if lower.contains("playlist") {
                    Resolved::Playlist(id)
                } else if lower.contains("album") {
                    Resolved::Album(id)
                } else {
                    Resolved::Song(id)
                });
            }
        }
    }
    if !text.is_empty() && text.len() <= 20 && text.chars().all(|c| c.is_ascii_digit()) {
        if let Ok(id) = text.parse() {
            return Ok(Resolved::Song(id));
        }
    }
    Ok(Resolved::Query(text.to_owned()))
}

pub async fn resolve(api: &Netease, text: &str) -> Result<Resolved> {
    match classify(text) {
        Ok(r) => Ok(r),
        Err(short) => {
            let full = api.expand_short_link(&short).await?;
            Ok(classify(&full).unwrap_or(Resolved::Query(text.trim().to_owned())))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_inputs() {
        assert_eq!(classify("186016"), Ok(Resolved::Song(186016)));
        assert_eq!(classify("https://music.163.com/song?id=186016&userid=1"), Ok(Resolved::Song(186016)));
        assert_eq!(classify("分享 周杰伦的单曲《晴天》: https://music.163.com/#/song?id=186016 (来自@网易云音乐)"), Ok(Resolved::Song(186016)));
        assert_eq!(classify("https://music.163.com/#/playlist?id=3778678"), Ok(Resolved::Playlist(3778678)));
        assert_eq!(classify("https://music.163.com/album?id=18905"), Ok(Resolved::Album(18905)));
        assert_eq!(classify("https://y.music.163.com/m/playlist/3778678"), Ok(Resolved::Playlist(3778678)));
        assert_eq!(classify("周杰伦 晴天"), Ok(Resolved::Query("周杰伦 晴天".into())));
        assert_eq!(classify("看 http://163cn.tv/abc 这个"), Err("http://163cn.tv/abc".into()));
    }
}
