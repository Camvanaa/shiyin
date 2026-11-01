//! 酷我音乐短信登录。三步，全部服务端完成，Web 版也能用：
//!
//! 1. `captcha/getcode`      取图形验证码（PNG data URL + token）
//! 2. `sms/mobileLoginCode`  用手机号 + 图形验证码换短信，返回 `tm`
//! 3. `login/loginByMobile`  手机号 + 短信验证码 + tm → `cookies` 键值对
//!
//! 第 2 步的 `tm` 和 `verifyCode`（图形验证码）在第 3 步仍需带上；服务端按
//! 手机号暂存这两项，前端只需要重新提交短信码。

use std::collections::HashMap;
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use crate::error::{Error, Result};
use crate::model::Captcha;
use crate::providers::kuwo::Kuwo;
use crate::providers::s;

const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";
/// 短信验证码本身约 5 分钟有效
const SESSION_TTL: Duration = Duration::from_secs(10 * 60);

/// 发送短信后到提交之间需要保留的上下文
pub struct SmsSession {
    pub mobile: String,
    pub verify_code: String,
    pub tm: String,
    created: Instant,
}

fn kuwo_error(v: &Value, default: &str) -> Error {
    let code = v["code"].as_i64().unwrap_or(-1);
    let msg = match code {
        -102 => "图形验证码错误".to_owned(),
        1107 => "短信验证码已过期".to_owned(),
        1108 => "短信验证码不正确".to_owned(),
        1100 => "手机号未注册".to_owned(),
        1157 => "操作过于频繁，请稍后再试".to_owned(),
        _ => {
            let m = s(&v["msg"]);
            if m.is_empty() { default.to_owned() } else { m }
        }
    };
    Error::Api { code, msg }
}

impl Kuwo {
    fn sms_lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, SmsSession>> {
        self.sms.lock().unwrap_or_else(|e| e.into_inner())
    }

    async fn post_json(&self, url: &str, body: Value) -> Result<Value> {
        let text = crate::source::Source::http(self)
            .post(url)
            .json(&body)
            .header(reqwest::header::USER_AGENT, UA)
            .header(reqwest::header::REFERER, "https://www.kuwo.cn/")
            .header(reqwest::header::ORIGIN, "https://www.kuwo.cn")
            .timeout(Duration::from_secs(20))
            .send()
            .await?
            .text()
            .await?;
        serde_json::from_str(&text).map_err(|_| Error::Parse("酷我返回非 JSON".into()))
    }

    /// 步骤 1：图形验证码
    pub(crate) async fn sms_captcha(&self) -> Result<Captcha> {
        let text = crate::source::Source::http(self)
            .get("https://wapi.kuwo.cn/api/common/captcha/getcode")
            .header(reqwest::header::USER_AGENT, UA)
            .header(reqwest::header::REFERER, "https://www.kuwo.cn/")
            .timeout(Duration::from_secs(15))
            .send()
            .await?
            .text()
            .await?;
        let v: Value = serde_json::from_str(&text).map_err(|_| Error::Parse("酷我验证码返回非 JSON".into()))?;
        if v["code"].as_i64() != Some(200) {
            return Err(kuwo_error(&v, "获取图形验证码失败"));
        }
        let token = s(&v["data"]["token"]);
        let image = s(&v["data"]["img"]);
        if token.is_empty() || image.is_empty() {
            return Err(Error::Parse("酷我未返回验证码".into()));
        }
        Ok(Captcha { token, image })
    }

    /// 步骤 2：发短信。成功后按手机号记住 tm 与图形验证码
    pub(crate) async fn sms_send(&self, mobile: &str, captcha_token: &str, captcha_code: &str) -> Result<()> {
        let mobile = mobile.trim();
        if mobile.len() != 11 || !mobile.chars().all(|c| c.is_ascii_digit()) {
            return Err(Error::Invalid("请输入 11 位手机号".into()));
        }
        let v = self
            .post_json(
                "https://wapi.kuwo.cn/api/sms/mobileLoginCode",
                json!({ "mobile": mobile, "verifyCode": captcha_code.trim(), "verifyCodeToken": captcha_token, "userIp": "" }),
            )
            .await?;
        if v["code"].as_i64() != Some(200) {
            return Err(kuwo_error(&v, "发送短信失败"));
        }
        let tm = s(&v["data"]["tm"]);
        let mut map = self.sms_lock();
        map.retain(|_, x| x.created.elapsed() < SESSION_TTL);
        map.insert(mobile.to_owned(), SmsSession { mobile: mobile.to_owned(), verify_code: captcha_code.trim().to_owned(), tm, created: Instant::now() });
        Ok(())
    }

    /// 步骤 3：提交短信验证码，返回整段 cookie
    pub(crate) async fn sms_login(&self, mobile: &str, sms_code: &str) -> Result<String> {
        let mobile = mobile.trim();
        let (verify_code, tm) = {
            let map = self.sms_lock();
            let sess = map.get(mobile).ok_or_else(|| Error::Invalid("请先获取短信验证码".into()))?;
            if sess.created.elapsed() > SESSION_TTL {
                return Err(Error::Invalid("短信验证码已过期，请重新获取".into()));
            }
            (sess.verify_code.clone(), sess.tm.clone())
        };
        let v = self
            .post_json(
                "https://wapi.kuwo.cn/api/www/login/loginByMobile",
                json!({ "mobile": mobile, "verifyCode": verify_code, "smsCode": sms_code.trim(), "tm": tm }),
            )
            .await?;
        if v["code"].as_i64() != Some(200) {
            return Err(kuwo_error(&v, "登录失败"));
        }
        let cookies = v["data"]["cookies"].as_object().ok_or_else(|| Error::Parse("酷我未返回登录凭证".into()))?;
        let joined: Vec<String> = cookies.iter().filter_map(|(k, val)| {
            let vs = s(val);
            (!vs.is_empty()).then(|| format!("{k}={vs}"))
        }).collect();
        if joined.is_empty() {
            return Err(Error::Parse("酷我登录凭证为空".into()));
        }
        self.sms_lock().remove(mobile);
        Ok(joined.join("; "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_known_error_codes() {
        let e = kuwo_error(&json!({ "code": -102, "msg": "picture captcha error" }), "x");
        assert!(e.to_string().contains("图形验证码"));
        let e = kuwo_error(&json!({ "code": 1108 }), "x");
        assert!(e.to_string().contains("短信验证码不正确"));
        let e = kuwo_error(&json!({ "code": 9999, "msg": "custom" }), "default");
        assert!(e.to_string().contains("custom"));
        let e = kuwo_error(&json!({ "code": 9999 }), "default");
        assert!(e.to_string().contains("default"));
    }
}
