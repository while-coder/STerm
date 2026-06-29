// 全局会话状态：每个 SSH 连接对应一个 SessionEntry，按前端生成的 id 索引。
use std::collections::HashMap;
use std::sync::Arc;

use russh::client;
use russh_sftp::client::SftpSession;
use tokio::sync::{mpsc, Mutex};

use crate::ssh::{ClientHandler, ShellCmd};

/// 单个 SSH 会话所持有的资源。
pub struct SessionEntry {
    /// SSH 连接句柄，用于按需打开新通道（如 SFTP 子系统）。
    pub handle: Arc<client::Handle<ClientHandler>>,
    /// 向 shell actor 任务发送输入 / resize / 关闭命令。
    pub shell_tx: mpsc::Sender<ShellCmd>,
    /// SFTP 会话懒加载缓存：首次文件操作时建立，后续复用。
    pub sftp: Mutex<Option<Arc<SftpSession>>>,
}

/// 应用级状态，由 Tauri 通过 `manage` 注入。
#[derive(Default)]
pub struct AppState {
    pub sessions: Mutex<HashMap<String, Arc<SessionEntry>>>,
}
