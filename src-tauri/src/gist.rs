// GitHub Gist 同步的 HTTP 搬运层：把前端加密好的密文上传/拉取到一个 secret gist。
// 加解密与合并都在前端，这里只负责 HTTPS 请求和从系统凭证库读取 PAT。

const PAT_KEY: &str = "github-pat";
const FILE_NAME: &str = "connections.enc";
const GIST_DESC: &str = "STerm 机器列表（加密同步，请勿手动修改）";

#[derive(serde::Serialize)]
pub struct GistPull {
    /// connections.enc 文件内容；gist 中不存在该文件时为 None。
    content: Option<String>,
    /// 最近一次提交的 version（commit sha），用于前端判断是否变化。
    version: Option<String>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GistPush {
    gist_id: String,
    version: Option<String>,
}

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn with_headers(req: reqwest::RequestBuilder, pat: &str) -> reqwest::RequestBuilder {
    req.header("User-Agent", "STerm")
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .header("Authorization", format!("Bearer {pat}"))
}

fn net_err(e: reqwest::Error) -> String {
    format!("网络请求失败：{e}")
}

fn require_pat() -> Result<String, String> {
    crate::credential::read_credential(PAT_KEY)?
        .ok_or_else(|| "尚未配置 GitHub PAT".to_string())
}

fn status_err(status: reqwest::StatusCode) -> String {
    match status.as_u16() {
        401 => "GitHub 认证失败：PAT 无效或已过期，请重新连接".to_string(),
        403 => "GitHub 拒绝请求：可能触发了速率限制或 PAT 缺少 gist 权限".to_string(),
        404 => "未找到该 Gist：可能已被删除或 PAT 无权访问".to_string(),
        other => format!("GitHub 接口返回错误：HTTP {other}"),
    }
}

fn parse_version(v: &serde_json::Value) -> Option<String> {
    v.get("history")
        .and_then(|h| h.as_array())
        .and_then(|a| a.first())
        .and_then(|e| e.get("version"))
        .and_then(|x| x.as_str())
        .map(|s| s.to_string())
}

/// 验证 PAT 是否有效，返回 GitHub 用户名（login）。
#[tauri::command]
pub async fn gist_validate(pat: String) -> Result<String, String> {
    let resp = with_headers(client().get("https://api.github.com/user"), &pat)
        .send()
        .await
        .map_err(net_err)?;
    if !resp.status().is_success() {
        return Err(status_err(resp.status()));
    }
    let v: serde_json::Value = resp.json().await.map_err(net_err)?;
    Ok(v.get("login")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string())
}

/// 拉取 gist 中的 connections.enc 内容与最新 version。
#[tauri::command]
pub async fn gist_pull(gist_id: String) -> Result<GistPull, String> {
    let pat = require_pat()?;
    let url = format!("https://api.github.com/gists/{gist_id}");
    let resp = with_headers(client().get(&url), &pat)
        .send()
        .await
        .map_err(net_err)?;
    if !resp.status().is_success() {
        return Err(status_err(resp.status()));
    }
    let v: serde_json::Value = resp.json().await.map_err(net_err)?;
    let version = parse_version(&v);

    let file = v.get("files").and_then(|f| f.get(FILE_NAME));
    let content = match file {
        None => None,
        Some(f) => {
            let truncated = f.get("truncated").and_then(|t| t.as_bool()).unwrap_or(false);
            if truncated {
                // 内容过大被截断时，从 raw_url 取完整内容（正常机器列表不会触发）。
                let raw_url = f
                    .get("raw_url")
                    .and_then(|u| u.as_str())
                    .ok_or_else(|| "Gist 文件被截断且缺少 raw_url".to_string())?;
                let raw = with_headers(client().get(raw_url), &pat)
                    .send()
                    .await
                    .map_err(net_err)?;
                if !raw.status().is_success() {
                    return Err(status_err(raw.status()));
                }
                Some(raw.text().await.map_err(net_err)?)
            } else {
                f.get("content").and_then(|c| c.as_str()).map(|s| s.to_string())
            }
        }
    };

    Ok(GistPull { content, version })
}

/// 推送 content 到 gist。gist_id 为空则新建一个 secret gist，返回其 id。
#[tauri::command]
pub async fn gist_push(gist_id: Option<String>, content: String) -> Result<GistPush, String> {
    let pat = require_pat()?;
    let files = serde_json::json!({ FILE_NAME: { "content": content } });

    let resp = match gist_id.as_deref().filter(|s| !s.is_empty()) {
        // 更新已有 gist。
        Some(id) => {
            let url = format!("https://api.github.com/gists/{id}");
            let body = serde_json::json!({ "description": GIST_DESC, "files": files });
            with_headers(client().patch(&url), &pat)
                .json(&body)
                .send()
                .await
                .map_err(net_err)?
        }
        // 新建 secret gist。
        None => {
            let body = serde_json::json!({
                "description": GIST_DESC,
                "public": false,
                "files": files,
            });
            with_headers(client().post("https://api.github.com/gists"), &pat)
                .json(&body)
                .send()
                .await
                .map_err(net_err)?
        }
    };

    if !resp.status().is_success() {
        return Err(status_err(resp.status()));
    }
    let v: serde_json::Value = resp.json().await.map_err(net_err)?;
    let id = v
        .get("id")
        .and_then(|x| x.as_str())
        .ok_or_else(|| "GitHub 返回数据缺少 gist id".to_string())?
        .to_string();
    Ok(GistPush {
        gist_id: id,
        version: parse_version(&v),
    })
}

/// 在当前 PAT 对应账号下查找已有的 STerm 同步 gist（含 connections.enc 文件）。
/// 多个匹配时取最早创建的，保证多设备稳定收敛到同一个；找不到返回 None。
#[tauri::command]
pub async fn gist_find() -> Result<Option<String>, String> {
    let pat = require_pat()?;
    // 仅取第一页（每个账号的 STerm gist 通常只有一个；超过 100 个 gist 时可能漏查）。
    let resp = with_headers(
        client().get("https://api.github.com/gists?per_page=100"),
        &pat,
    )
    .send()
    .await
    .map_err(net_err)?;
    if !resp.status().is_success() {
        return Err(status_err(resp.status()));
    }
    let list: serde_json::Value = resp.json().await.map_err(net_err)?;
    let Some(arr) = list.as_array() else {
        return Ok(None);
    };

    let mut best: Option<(&str, &str)> = None; // (created_at, id)
    for g in arr {
        let has_file = g
            .get("files")
            .and_then(|f| f.get(FILE_NAME))
            .is_some();
        if !has_file {
            continue;
        }
        let id = g.get("id").and_then(|x| x.as_str());
        let created = g.get("created_at").and_then(|x| x.as_str()).unwrap_or("");
        if let Some(id) = id {
            match best {
                // created_at 为 ISO8601，可直接字典序比较取最早。
                Some((c, _)) if created >= c => {}
                _ => best = Some((created, id)),
            }
        }
    }
    Ok(best.map(|(_, id)| id.to_string()))
}
