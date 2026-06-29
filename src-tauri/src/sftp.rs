// SFTP 文件操作：复用已建立的 SSH 连接，按需打开 sftp 子系统通道。

use std::sync::Arc;

use anyhow::Result;
use russh_sftp::client::SftpSession;
use serde::Serialize;
use tauri::State;

use crate::state::{AppState, SessionEntry};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    /// 修改时间（Unix 秒）。
    pub modified: u64,
}

/// 从会话表取出会话条目（克隆 Arc 后立即释放表锁）。
async fn fetch_entry(state: &State<'_, AppState>, id: &str) -> Result<Arc<SessionEntry>, String> {
    let sessions = state.sessions.lock().await;
    sessions
        .get(id)
        .cloned()
        .ok_or_else(|| "会话不存在".to_string())
}

/// 取得（或懒加载建立）该会话的 SFTP 子系统。
async fn get_sftp(entry: &SessionEntry) -> Result<Arc<SftpSession>> {
    let mut guard = entry.sftp.lock().await;
    if let Some(s) = guard.as_ref() {
        return Ok(s.clone());
    }
    let channel = entry.handle.channel_open_session().await?;
    channel.request_subsystem(true, "sftp").await?;
    let sftp = SftpSession::new(channel.into_stream()).await?;
    let arc = Arc::new(sftp);
    *guard = Some(arc.clone());
    Ok(arc)
}

fn join_path(base: &str, name: &str) -> String {
    if base.ends_with('/') {
        format!("{base}{name}")
    } else {
        format!("{base}/{name}")
    }
}

#[tauri::command]
pub async fn sftp_home(state: State<'_, AppState>, id: String) -> Result<String, String> {
    let entry = fetch_entry(&state, &id).await?;
    let sftp = get_sftp(&entry).await.map_err(|e| e.to_string())?;
    sftp.canonicalize(".").await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn sftp_list(
    state: State<'_, AppState>,
    id: String,
    path: String,
) -> Result<Vec<FileEntry>, String> {
    let entry = fetch_entry(&state, &id).await?;
    list_inner(&entry, &path).await.map_err(|e| e.to_string())
}

async fn list_inner(entry: &SessionEntry, path: &str) -> Result<Vec<FileEntry>> {
    let sftp = get_sftp(entry).await?;
    let mut out = Vec::new();
    for item in sftp.read_dir(path).await? {
        let name = item.file_name();
        let meta = item.metadata();
        out.push(FileEntry {
            path: join_path(path, &name),
            is_dir: meta.is_dir(),
            size: meta.size.unwrap_or(0),
            modified: meta.mtime.unwrap_or(0) as u64,
            name,
        });
    }
    // 目录在前，再按名称排序。
    out.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then(a.name.cmp(&b.name)));
    Ok(out)
}

#[tauri::command]
pub async fn sftp_download(
    state: State<'_, AppState>,
    id: String,
    remote_path: String,
    local_path: String,
) -> Result<(), String> {
    let entry = fetch_entry(&state, &id).await?;
    download_inner(&entry, &remote_path, &local_path)
        .await
        .map_err(|e| e.to_string())
}

async fn download_inner(entry: &SessionEntry, remote: &str, local: &str) -> Result<()> {
    let sftp = get_sftp(entry).await?;
    let mut src = sftp.open(remote).await?;
    let mut dst = tokio::fs::File::create(local).await?;
    tokio::io::copy(&mut src, &mut dst).await?;
    Ok(())
}

#[tauri::command]
pub async fn sftp_upload(
    state: State<'_, AppState>,
    id: String,
    local_path: String,
    remote_path: String,
) -> Result<(), String> {
    let entry = fetch_entry(&state, &id).await?;
    upload_inner(&entry, &local_path, &remote_path)
        .await
        .map_err(|e| e.to_string())
}

async fn upload_inner(entry: &SessionEntry, local: &str, remote: &str) -> Result<()> {
    use tokio::io::AsyncWriteExt;
    let sftp = get_sftp(entry).await?;
    let mut src = tokio::fs::File::open(local).await?;
    let mut dst = sftp.create(remote).await?;
    tokio::io::copy(&mut src, &mut dst).await?;
    dst.shutdown().await?; // 确保刷新并关闭远端文件
    Ok(())
}
