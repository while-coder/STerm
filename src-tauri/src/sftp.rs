// SFTP 文件操作：复用已建立的 SSH 连接，按需打开 sftp 子系统通道。

use std::path::{Path, PathBuf};
#[cfg(not(target_os = "windows"))]
use std::process::Command;
use std::sync::Arc;

use anyhow::Result;
use russh_sftp::client::SftpSession;
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use tokio_util::sync::CancellationToken;

use crate::state::{AppState, SessionEntry};

/// 注册传输取消令牌，返回克隆供拷贝循环检查。
async fn register_transfer(state: &State<'_, AppState>, transfer_id: &str) -> CancellationToken {
    let token = CancellationToken::new();
    state
        .transfers
        .lock()
        .await
        .insert(transfer_id.to_string(), token.clone());
    token
}

async fn unregister_transfer(state: &State<'_, AppState>, transfer_id: &str) {
    state.transfers.lock().await.remove(transfer_id);
}

/// 下载进度事件载荷（事件名 `sftp-progress-<transferId>`）。
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Progress {
    transferred: u64,
    total: u64,
    done: bool,
}

/// 分块拷贝并按阈值上报进度。`done` 为累计已传字节（跨多文件累加用）。
/// 返回 Ok(true) 表示完成，Ok(false) 表示被取消令牌中断。
async fn copy_emit<R, W>(
    src: &mut R,
    dst: &mut W,
    total: u64,
    done: &mut u64,
    expected_bytes: Option<u64>,
    app: &AppHandle,
    event: &str,
    token: &CancellationToken,
) -> Result<bool>
where
    R: tokio::io::AsyncRead + Unpin,
    W: tokio::io::AsyncWrite + Unpin,
{
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut buf = vec![0u8; 64 * 1024];
    let mut last = *done;
    let mut file_done = 0u64;
    loop {
        if token.is_cancelled() {
            return Ok(false);
        }
        let read_len = match expected_bytes {
            Some(expected) if file_done >= expected => break,
            Some(expected) => {
                usize::try_from((expected - file_done).min(buf.len() as u64)).unwrap_or(buf.len())
            }
            None => buf.len(),
        };
        if read_len == 0 {
            break;
        }
        let n = tokio::select! {
            _ = token.cancelled() => return Ok(false),
            res = src.read(&mut buf[..read_len]) => res?,
        };
        if n == 0 {
            break;
        }
        tokio::select! {
            _ = token.cancelled() => return Ok(false),
            res = dst.write_all(&buf[..n]) => res?,
        };
        *done += n as u64;
        file_done += n as u64;
        // 每累计约 512KB 上报一次，避免事件风暴。
        if *done - last >= 512 * 1024 {
            last = *done;
            let _ = app.emit(
                event,
                Progress {
                    transferred: *done,
                    total,
                    done: false,
                },
            );
        }
    }
    tokio::select! {
        _ = token.cancelled() => return Ok(false),
        res = dst.flush() => res?,
    };
    Ok(true)
}

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

/// 取消进行中的传输（按 transferId）。
#[tauri::command]
pub async fn sftp_cancel(state: State<'_, AppState>, transfer_id: String) -> Result<(), String> {
    if let Some(token) = state.transfers.lock().await.get(&transfer_id) {
        token.cancel();
    }
    Ok(())
}

#[tauri::command]
pub async fn sftp_download(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    remote_path: String,
    local_path: String,
    transfer_id: String,
) -> Result<(), String> {
    let entry = fetch_entry(&state, &id).await?;
    let token = register_transfer(&state, &transfer_id).await;
    let res = download_inner(
        &app,
        &entry,
        &remote_path,
        &local_path,
        &transfer_id,
        &token,
    )
    .await;
    unregister_transfer(&state, &transfer_id).await;
    match res {
        Ok(true) => Ok(()),
        Ok(false) => Err("已取消".to_string()),
        Err(e) => Err(e.to_string()),
    }
}

async fn download_inner(
    app: &AppHandle,
    entry: &SessionEntry,
    remote: &str,
    local: &str,
    transfer_id: &str,
    token: &CancellationToken,
) -> Result<bool> {
    let sftp = get_sftp(entry).await?;
    let event = format!("sftp-progress-{transfer_id}");
    let expected = sftp.metadata(remote).await.ok().and_then(|m| m.size);
    let total = expected.unwrap_or(0);
    let mut src = sftp.open(remote).await?;
    let mut dst = tokio::fs::File::create(local).await?;
    let mut done = 0u64;
    let completed = copy_emit(
        &mut src, &mut dst, total, &mut done, expected, app, &event, token,
    )
    .await?;
    if !completed {
        drop(dst);
        let _ = tokio::fs::remove_file(local).await; // 清理半成品
        return Ok(false);
    }
    let _ = app.emit(
        &event,
        Progress {
            transferred: done,
            total,
            done: true,
        },
    );
    Ok(true)
}

#[tauri::command]
pub async fn sftp_upload(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    local_path: String,
    remote_path: String,
    transfer_id: String,
) -> Result<(), String> {
    let entry = fetch_entry(&state, &id).await?;
    let token = register_transfer(&state, &transfer_id).await;
    let res = upload_inner(
        &app,
        &entry,
        &local_path,
        &remote_path,
        &transfer_id,
        &token,
    )
    .await;
    unregister_transfer(&state, &transfer_id).await;
    match res {
        Ok(true) => Ok(()),
        Ok(false) => Err("已取消".to_string()),
        Err(e) => Err(e.to_string()),
    }
}

async fn upload_inner(
    app: &AppHandle,
    entry: &SessionEntry,
    local: &str,
    remote: &str,
    transfer_id: &str,
    token: &CancellationToken,
) -> Result<bool> {
    use tokio::io::AsyncWriteExt;
    let sftp = get_sftp(entry).await?;
    let event = format!("sftp-progress-{transfer_id}");
    let expected = tokio::fs::metadata(local).await.ok().map(|m| m.len());
    let total = expected.unwrap_or(0);
    let mut src = tokio::fs::File::open(local).await?;
    let mut dst = sftp.create(remote).await?;
    let mut done = 0u64;
    let completed = copy_emit(
        &mut src, &mut dst, total, &mut done, expected, app, &event, token,
    )
    .await?;
    if !completed {
        let _ = dst.shutdown().await;
        let _ = sftp.remove_file(remote).await; // 清理半成品
        return Ok(false);
    }
    dst.shutdown().await?; // 确保刷新并关闭远端文件
    let _ = app.emit(
        &event,
        Progress {
            transferred: done,
            total,
            done: true,
        },
    );
    Ok(true)
}

#[tauri::command]
pub async fn sftp_download_dir(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    remote_path: String,
    local_path: String,
    transfer_id: String,
) -> Result<(), String> {
    let entry = fetch_entry(&state, &id).await?;
    let token = register_transfer(&state, &transfer_id).await;
    let res = download_dir_inner(
        &app,
        &entry,
        &remote_path,
        &local_path,
        &transfer_id,
        &token,
    )
    .await;
    unregister_transfer(&state, &transfer_id).await;
    match res {
        Ok(true) => Ok(()),
        Ok(false) => Err("已取消".to_string()),
        Err(e) => Err(e.to_string()),
    }
}

/// 递归下载远端目录（迭代式，避免 async 递归装箱）：先遍历求总字节数并建好本地目录，
/// 再逐文件分块拷贝、按总量累计上报进度。
async fn download_dir_inner(
    app: &AppHandle,
    entry: &SessionEntry,
    remote_root: &str,
    local_root: &str,
    transfer_id: &str,
    token: &CancellationToken,
) -> Result<bool> {
    let sftp = get_sftp(entry).await?;
    let event = format!("sftp-progress-{transfer_id}");

    // 1) 遍历：建本地目录、收集文件清单与总字节。
    let mut files: Vec<(String, PathBuf, u64)> = Vec::new();
    let mut total = 0u64;
    let mut stack: Vec<(String, PathBuf)> =
        vec![(remote_root.to_string(), PathBuf::from(local_root))];
    while let Some((remote, local)) = stack.pop() {
        tokio::fs::create_dir_all(&local).await?;
        for item in sftp.read_dir(&remote).await? {
            let name = item.file_name();
            let meta = item.metadata();
            let remote_child = join_path(&remote, &name);
            let local_child = local.join(&name);
            if meta.is_dir() {
                stack.push((remote_child, local_child));
            } else {
                let size = meta.size.unwrap_or(0);
                total += size;
                files.push((remote_child, local_child, size));
            }
        }
    }

    // 2) 逐文件拷贝，done 跨文件累加。
    let mut done = 0u64;
    for (remote_child, local_child, size) in files {
        let mut src = sftp.open(&remote_child).await?;
        let mut dst = tokio::fs::File::create(&local_child).await?;
        let completed = copy_emit(
            &mut src,
            &mut dst,
            total,
            &mut done,
            Some(size),
            app,
            &event,
            token,
        )
        .await?;
        if !completed {
            drop(dst);
            let _ = tokio::fs::remove_file(&local_child).await; // 清理半成品（已完成文件保留）
            return Ok(false);
        }
    }
    let _ = app.emit(
        &event,
        Progress {
            transferred: done,
            total,
            done: true,
        },
    );
    Ok(true)
}

#[tauri::command]
pub async fn sftp_mkdir(
    state: State<'_, AppState>,
    id: String,
    path: String,
) -> Result<(), String> {
    let entry = fetch_entry(&state, &id).await?;
    let sftp = get_sftp(&entry).await.map_err(|e| e.to_string())?;
    sftp.create_dir(path).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn sftp_rename(
    state: State<'_, AppState>,
    id: String,
    from: String,
    to: String,
) -> Result<(), String> {
    let entry = fetch_entry(&state, &id).await?;
    let sftp = get_sftp(&entry).await.map_err(|e| e.to_string())?;
    sftp.rename(from, to).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn sftp_remove(
    state: State<'_, AppState>,
    id: String,
    path: String,
    is_dir: bool,
) -> Result<(), String> {
    let entry = fetch_entry(&state, &id).await?;
    let sftp = get_sftp(&entry).await.map_err(|e| e.to_string())?;
    if is_dir {
        sftp.remove_dir(path).await.map_err(|e| e.to_string())
    } else {
        sftp.remove_file(path).await.map_err(|e| e.to_string())
    }
}

/// 确保本地目录存在（递归创建），供双击查看时准备缓存目录。
#[tauri::command]
pub async fn ensure_dir(path: String) -> Result<(), String> {
    tokio::fs::create_dir_all(&path)
        .await
        .map_err(|e| e.to_string())
}

/// 确保本地目录存在并用系统文件管理器打开。
#[tauri::command]
pub async fn open_dir(path: String) -> Result<(), String> {
    tokio::fs::create_dir_all(&path)
        .await
        .map_err(|e| format!("创建目录失败：{e}"))?;
    open_local_path_inner(Path::new(&path))
}

/// 用系统默认程序打开本地文件或目录。
#[tauri::command]
pub async fn open_local_path(path: String) -> Result<(), String> {
    tokio::fs::metadata(&path)
        .await
        .map_err(|e| format!("本地路径不可访问：{e}"))?;
    open_local_path_inner(Path::new(&path))
}

#[cfg(target_os = "windows")]
fn open_local_path_inner(path: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let file: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            std::ptr::null(),
            file.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        )
    };
    if result as isize <= 32 {
        return Err(format!(
            "打开本地路径失败：ShellExecuteW 错误 {}",
            result as isize
        ));
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn open_local_path_inner(path: &Path) -> Result<(), String> {
    Command::new("open")
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("打开本地路径失败：{e}"))
}

#[cfg(all(unix, not(target_os = "macos")))]
fn open_local_path_inner(path: &Path) -> Result<(), String> {
    Command::new("xdg-open")
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("打开本地路径失败：{e}"))
}
