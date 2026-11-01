//! QQ 音乐扫码登录。腾讯统一登录（ptlogin）→ QQ 互联 OAuth → QQ 音乐 musickey，共五步：
//!
//! 1. `ptqrshow`   取二维码 PNG 与 `qrsig` cookie
//! 2. `ptqrlogin`  轮询扫码状态（`ptqrtoken = hash33(qrsig)`）；成功时返回 check_sig 跳转地址
//! 3. `check_sig`  跟随跳转拿到 `p_skey` 等整套 cookie
//! 4. `oauth2.0/authorize` POST 取 Location 里的授权 `code`（`g_tk = hash33(p_skey, 5381)`）
//! 5. `QQConnectLogin.LoginServer/QQLogin`（musics.fcg，需 zzc 签名）用 code 换 `musickey`
//!
//! 参考 qqmusic-api-python 与 music-together 的实现；`music.login.LoginServer/Login`
//! 已返回 104400，必须用 `QQConnectLogin`。

use std::collections::HashMap;
use std::time::{Duration, Instant};

use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use serde_json::{json, Value};
use sha1::{Digest, Sha1};

use crate::error::{Error, Result};
use crate::model::{QrStart, QrState};
use crate::providers::qq::QQ;
use crate::providers::s;

const APPID: &str = "716027609";
const QQ_MUSIC_OAUTH_APPID: &str = "100497308";
const LOGIN_JUMP: &str = "https://graph.qq.com/oauth2.0/login_jump";
const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";
/// 二维码会话保留时长；ptlogin 二维码本身约 2 分钟过期
const SESSION_TTL: Duration = Duration::from_secs(5 * 60);

pub struct QrSession {
    /// 跨请求累积的 cookie（qrsig、pt_login_sig、p_skey...）
    cookies: HashMap<String, String>,
    created: Instant,
    /// 登录成功后暂存结果，避免重复轮询时二次执行换 key
    done: Option<String>,
}

impl QrSession {
    fn cookie_header(&self) -> String {
        self.cookies.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join("; ")
    }

    fn absorb(&mut self, resp: &reqwest::Response) {
        for h in resp.headers().get_all(reqwest::header::SET_COOKIE) {
            let Ok(c) = h.to_str() else { continue };
            let Some((k, v)) = c.split(';').next().and_then(|kv| kv.split_once('=')) else { continue };
            let (k, v) = (k.trim(), v.trim());
            // 服务端清 cookie 会下发空值，跳过
            if !k.is_empty() && !v.is_empty() {
                self.cookies.insert(k.to_owned(), v.to_owned());
            }
        }
    }
}

/// 腾讯 hash33：ptqrtoken（初值 0）与 g_tk（初值 5381）。
/// 前端用 JS Number 累加（`hash += (hash << 5) + code`），`<<` 先把 hash 截成 int32，
/// 但相加结果保持 float64 不回绕；最后 `& 0x7fffffff`。这里逐步复刻。
fn hash33(input: &str, init: i64) -> i64 {
    let mut h: i64 = init;
    for b in input.bytes() {
        let shifted = ((h as i32).wrapping_shl(5)) as i64;
        h += shifted + b as i64;
    }
    // JS `&` 先把 float64 转 int32 再按位与
    ((h as i32) as i64) & 0x7fff_ffff
}

/// musics.fcg 的 zzc 请求签名（移植自 qqmusic-api-python sign.py）
fn zzc_sign(body: &str) -> String {
    const PART1: [usize; 8] = [23, 14, 6, 36, 16, 40, 7, 19];
    const PART2: [usize; 8] = [16, 1, 32, 12, 19, 27, 8, 5];
    const SCRAMBLE: [u8; 20] = [89, 39, 179, 150, 218, 82, 58, 252, 177, 52, 186, 123, 120, 64, 242, 133, 143, 161, 121, 179];
    let hex: Vec<char> = format!("{:X}", Sha1::digest(body.as_bytes())).chars().collect();
    let pick = |idx: &[usize]| idx.iter().filter(|&&i| i < hex.len()).map(|&i| hex[i]).collect::<String>();
    let mut part3 = [0u8; 20];
    for (i, v) in SCRAMBLE.iter().enumerate() {
        let byte = u8::from_str_radix(&hex[i * 2..i * 2 + 2].iter().collect::<String>(), 16).unwrap_or(0);
        part3[i] = v ^ byte;
    }
    let b64: String = B64.encode(part3).chars().filter(|c| !matches!(c, '/' | '+' | '=')).collect();
    format!("zzc{}{}{}", pick(&PART1), b64, pick(&PART2)).to_lowercase()
}

fn parse_ptui_cb(text: &str) -> Option<Vec<String>> {
    let inner = text.trim().strip_prefix("ptuiCB(")?.strip_suffix(')')?;
    // 参数用单引号包裹，分隔符可能是 `','` 或 `', '`
    let mut out = Vec::new();
    let mut rest = inner.trim();
    while let Some(start) = rest.find('\'') {
        let after = &rest[start + 1..];
        let Some(end) = after.find('\'') else { break };
        out.push(after[..end].to_owned());
        rest = &after[end + 1..];
    }
    (!out.is_empty()).then_some(out)
}

fn query_val<'a>(url: &'a str, key: &str) -> Option<&'a str> {
    let q = url.split_once('?')?.1;
    q.split('&').find_map(|kv| kv.strip_prefix(key).and_then(|r| r.strip_prefix('=')))
}

impl QQ {
    fn qr_lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, QrSession>> {
        self.qr.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub(crate) async fn qq_qr_start(&self) -> Result<QrStart> {
        let resp = self
            .raw
            .get("https://ssl.ptlogin2.qq.com/ptqrshow")
            .query(&[
                ("appid", APPID),
                ("e", "2"),
                ("l", "M"),
                ("s", "3"),
                ("d", "72"),
                ("v", "4"),
                ("t", "0.1"),
                ("daid", "383"),
                ("pt_3rd_aid", QQ_MUSIC_OAUTH_APPID),
                ("u1", LOGIN_JUMP),
            ])
            .header(reqwest::header::USER_AGENT, UA)
            .header(reqwest::header::REFERER, "https://xui.ptlogin2.qq.com/")
            .timeout(Duration::from_secs(15))
            .send()
            .await?;
        let mut session = QrSession { cookies: HashMap::new(), created: Instant::now(), done: None };
        session.absorb(&resp);
        let qrsig = session.cookies.get("qrsig").cloned().ok_or_else(|| Error::Parse("QQ 未返回二维码签名".into()))?;
        let png = resp.bytes().await?;
        if png.len() < 100 {
            return Err(Error::Parse("QQ 二维码图片无效".into()));
        }
        let image = format!("data:image/png;base64,{}", B64.encode(&png));
        {
            let mut map = self.qr_lock();
            map.retain(|_, v| v.created.elapsed() < SESSION_TTL);
            map.insert(qrsig.clone(), session);
        }
        Ok(QrStart { key: qrsig, url: String::new(), svg: String::new(), image })
    }

    pub(crate) async fn qq_qr_check(&self, qrsig: &str) -> Result<(QrState, Option<String>)> {
        let (cookie, done) = {
            let map = self.qr_lock();
            let Some(sess) = map.get(qrsig) else { return Ok((QrState::Expired, None)) };
            (sess.cookie_header(), sess.done.clone())
        };
        if let Some(c) = done {
            return Ok((QrState::Success, Some(c)));
        }
        let ptqrtoken = hash33(qrsig, 0).to_string();
        let action = format!("0-0-{}", now_ms());
        let resp = self
            .raw
            .get("https://ssl.ptlogin2.qq.com/ptqrlogin")
            .query(&[
                ("u1", LOGIN_JUMP),
                ("ptqrtoken", ptqrtoken.as_str()),
                ("ptredirect", "1"),
                ("h", "1"),
                ("t", "1"),
                ("g", "1"),
                ("from_ui", "1"),
                ("ptlang", "2052"),
                ("action", action.as_str()),
                ("js_ver", "24112817"),
                ("js_type", "1"),
                ("pt_uistyle", "40"),
                ("aid", APPID),
                ("daid", "383"),
                ("pt_3rd_aid", QQ_MUSIC_OAUTH_APPID),
                ("has_resolve", "1"),
            ])
            .header(reqwest::header::USER_AGENT, UA)
            .header(reqwest::header::REFERER, "https://xui.ptlogin2.qq.com/")
            .header(reqwest::header::COOKIE, &cookie)
            .timeout(Duration::from_secs(15))
            .send()
            .await?;
        {
            let mut map = self.qr_lock();
            if let Some(sess) = map.get_mut(qrsig) {
                sess.absorb(&resp);
            }
        }
        let text = resp.text().await?;
        let parts = parse_ptui_cb(&text).ok_or_else(|| Error::Parse(format!("QQ 扫码状态格式异常：{}", text.chars().take(80).collect::<String>())))?;
        let code = parts.first().map(String::as_str).unwrap_or("");
        match code {
            "66" => Ok((QrState::Waiting, None)),
            "67" => Ok((QrState::Scanned, None)),
            "65" => {
                self.qr_lock().remove(qrsig);
                Ok((QrState::Expired, None))
            }
            "0" => {
                let check_sig_url = parts.get(2).cloned().unwrap_or_default();
                let cookie = self.finish_login(qrsig, &check_sig_url).await?;
                let mut map = self.qr_lock();
                if let Some(sess) = map.get_mut(qrsig) {
                    sess.done = Some(cookie.clone());
                }
                Ok((QrState::Success, Some(cookie)))
            }
            other => Err(Error::Api { code: other.parse().unwrap_or(-1), msg: parts.get(4).cloned().unwrap_or_else(|| "扫码失败".into()) }),
        }
    }

    /// 步骤 3-5：check_sig → OAuth code → musickey
    async fn finish_login(&self, qrsig: &str, check_sig_url: &str) -> Result<String> {
        let uin = query_val(check_sig_url, "uin").ok_or_else(|| Error::Parse("QQ 登录跳转缺少 uin".into()))?.to_owned();
        let ptsigx = query_val(check_sig_url, "ptsigx").ok_or_else(|| Error::Parse("QQ 登录跳转缺少 ptsigx".into()))?.to_owned();

        // 3. check_sig：拿 p_skey
        let cookie = self.qr_lock().get(qrsig).map(QrSession::cookie_header).unwrap_or_default();
        let resp = self
            .raw
            .get("https://ssl.ptlogin2.graph.qq.com/check_sig")
            .query(&[
                ("uin", uin.as_str()),
                ("pttype", "1"),
                ("service", "ptqrlogin"),
                ("nodirect", "0"),
                ("ptsigx", ptsigx.as_str()),
                ("s_url", LOGIN_JUMP),
                ("ptlang", "2052"),
                ("ptredirect", "100"),
                ("aid", APPID),
                ("daid", "383"),
                ("j_later", "0"),
                ("low_login_hour", "0"),
                ("regmaster", "0"),
                ("pt_login_type", "3"),
                ("pt_aid", "0"),
                ("pt_aaid", "16"),
                ("pt_light", "0"),
                ("pt_3rd_aid", QQ_MUSIC_OAUTH_APPID),
            ])
            .header(reqwest::header::USER_AGENT, UA)
            .header(reqwest::header::REFERER, "https://xui.ptlogin2.qq.com/")
            .header(reqwest::header::COOKIE, &cookie)
            .timeout(Duration::from_secs(15))
            .send()
            .await?;
        let (cookie, p_skey) = {
            let mut map = self.qr_lock();
            let sess = map.get_mut(qrsig).ok_or_else(|| Error::Invalid("扫码会话已失效".into()))?;
            sess.absorb(&resp);
            (sess.cookie_header(), sess.cookies.get("p_skey").cloned().unwrap_or_default())
        };
        if p_skey.is_empty() {
            return Err(Error::Parse("QQ 登录未返回 p_skey".into()));
        }

        // 4. OAuth authorize（POST，不跟随跳转，从 Location 取 code）
        let g_tk = hash33(&p_skey, 5381).to_string();
        let auth_time = now_ms().to_string();
        let ui = uuid_v4();
        let form = [
            ("response_type", "code"),
            ("client_id", QQ_MUSIC_OAUTH_APPID),
            ("redirect_uri", "https://y.qq.com/portal/wx_redirect.html?login_type=1&surl=https://y.qq.com/"),
            ("scope", "get_user_info,get_app_friends"),
            ("state", "state"),
            ("switch", ""),
            ("from_ptlogin", "1"),
            ("src", "1"),
            ("update_auth", "1"),
            ("openapi", "1010_1030"),
            ("g_tk", g_tk.as_str()),
            ("auth_time", auth_time.as_str()),
            ("ui", ui.as_str()),
        ];
        let resp = self
            .raw
            .post("https://graph.qq.com/oauth2.0/authorize")
            .form(&form)
            .header(reqwest::header::USER_AGENT, UA)
            .header(reqwest::header::COOKIE, &cookie)
            .timeout(Duration::from_secs(15))
            .send()
            .await?;
        let location = resp.headers().get(reqwest::header::LOCATION).and_then(|l| l.to_str().ok()).unwrap_or("").to_owned();
        let code = query_val(&location, "code")
            .map(str::to_owned)
            .ok_or_else(|| Error::Parse(format!("QQ 授权未返回 code（{}）", if location.is_empty() { "无跳转" } else { "跳转地址无 code" })))?;

        // 5. QQConnectLogin：code → musickey
        let body = json!({
            "comm": { "cv": 4747474, "ct": 24, "format": "json", "inCharset": "utf-8", "outCharset": "utf-8", "notice": 0, "tmeLoginType": "2" },
            "req": { "module": "QQConnectLogin.LoginServer", "method": "QQLogin", "param": { "code": code } }
        });
        let body_str = serde_json::to_string(&body)?;
        let sign = zzc_sign(&body_str);
        let v: Value = self
            .raw
            .post(format!("https://u.y.qq.com/cgi-bin/musics.fcg?sign={sign}"))
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .header(reqwest::header::USER_AGENT, UA)
            .header(reqwest::header::REFERER, "https://y.qq.com/")
            .body(body_str)
            .timeout(Duration::from_secs(15))
            .send()
            .await?
            .json()
            .await?;
        let req = &v["req"];
        let rc = req["code"].as_i64().unwrap_or(-1);
        if rc != 0 {
            return Err(Error::Api { code: rc, msg: format!("QQ 音乐登录失败：{}", s(&req["data"]["errMsg"])) });
        }
        let d = &req["data"];
        let musickey = s(&d["musickey"]);
        let musicid = s(&d["musicid"]);
        if musickey.is_empty() || musicid.is_empty() || musicid == "0" {
            return Err(Error::Parse("QQ 音乐未返回 musickey".into()));
        }
        let refresh = s(&d["refresh_token"]);
        Ok(format!("uin={musicid}; qqmusic_key={musickey}; qm_keyst={musickey}; o_cookie={musicid}; o_refresh_token={refresh}"))
    }
}

fn now_ms() -> u128 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0)
}

fn uuid_v4() -> String {
    use rand::RngCore;
    let mut b = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut b);
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7], b[8], b[9], b[10], b[11], b[12], b[13], b[14], b[15]
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash33_matches_reference() {
        // 与 ptlogin 前端 $.str.hash33 一致：实测该 qrsig 对应 ptqrtoken 456076991，ptqrlogin 返回 66
        assert_eq!(
            hash33("8ab6db3fb2492422503de2b15c33452f0c2631d25e4157f6f4833add2268d1fab1321052e0efcc58a2b615a0aa14e1cc342c503f519c9dbcb928532c3e3060f5", 0),
            456076991
        );
        assert_eq!(hash33("", 5381), 5381);
    }

    #[test]
    fn zzc_sign_shape() {
        let sign = zzc_sign(r#"{"comm":{"ct":24},"req":{"module":"x","method":"y","param":{}}}"#);
        assert!(sign.starts_with("zzc"));
        assert!(sign.len() > 30);
        assert!(sign.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()));
        // 确定性
        assert_eq!(sign, zzc_sign(r#"{"comm":{"ct":24},"req":{"module":"x","method":"y","param":{}}}"#));
    }

    #[test]
    fn parses_ptui_cb() {
        let p = parse_ptui_cb("ptuiCB('66','0','','0','二维码未失效。', '')").unwrap();
        assert_eq!(p[0], "66");
        assert_eq!(p[4], "二维码未失效。");
        let p = parse_ptui_cb("ptuiCB('0','0','https://ssl.ptlogin2.graph.qq.com/check_sig?pttype=1&uin=12345&service=ptqrlogin&ptsigx=abc','0','登录成功！', 'nick')").unwrap();
        assert_eq!(p[0], "0");
        assert_eq!(query_val(&p[2], "uin"), Some("12345"));
        assert_eq!(query_val(&p[2], "ptsigx"), Some("abc"));
        assert_eq!(p[5], "nick");
        assert!(parse_ptui_cb("garbage").is_none());
    }

    #[test]
    fn uuid_shape() {
        let u = uuid_v4();
        assert_eq!(u.len(), 36);
        assert_eq!(&u[14..15], "4");
    }
}
