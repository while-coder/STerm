// SSH 连接与交互式 PTY shell。
//
// 设计：每条 shell 通道由一个独立的 actor 任务持有。任务用 `tokio::select!`
// 同时（a）从通道读取输出并通过 Tauri 事件推送给前端，（b）从 mpsc 通道接收
// 前端的输入 / resize / 关闭命令。这样避免了在 `select!` 中同时可变借用 +
// 不可变借用同一个 Channel —— 通过 `Channel::split()` 拆成读写两半实现。

use std::{env, path::PathBuf, sync::Arc};

use anyhow::{anyhow, Result};
use base64::Engine;
use russh::client::{self, AuthResult};
use russh::keys::{load_secret_key, PrivateKeyWithHashAlg};
use russh::ChannelMsg;
use serde::Deserialize;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::mpsc;

use crate::state::{AppState, SessionEntry};

/// 发送给 shell actor 任务的命令。
pub enum ShellCmd {
    /// 写入用户输入（按键 / 粘贴）。
    Data(Vec<u8>),
    /// 终端尺寸变化（列, 行）。
    Resize(u32, u32),
    /// 关闭通道并结束任务。
    Close,
}

/// 连接建立后注入的 shell 配置：让 bash / zsh 在每个提示符前用 OSC 7
/// 转义序列汇报真实工作目录（`ESC ] 7 ; file://host/path BEL`），供前端捕获以驱动
/// SFTP 跟随。`printf` 中的 `\033` / `\a` 由远端 shell 解释为 ESC / BEL（故用裸反斜杠，
/// 这里用 raw string 保留）。行首空格尽量避免该命令进入 shell 历史。幂等：已注入则不重复追加。
/// 末尾的 clear 把这段注入命令的回显（及登录 banner）一次性擦掉，避免污染首屏。
const OSC7_SETUP: &str = concat!(
    r#" __sterm7(){ printf '\033]7;file://%s%s\a' "${HOSTNAME:-localhost}" "$PWD"; }; "#,
    r#"if [ -n "$ZSH_VERSION" ]; then typeset -ga precmd_functions; precmd_functions+=(__sterm7); "#,
    r#"elif [ -n "$BASH_VERSION" ]; then case "$PROMPT_COMMAND" in *__sterm7*) ;; *) "#,
    r#"PROMPT_COMMAND="__sterm7${PROMPT_COMMAND:+;$PROMPT_COMMAND}";; esac; fi"#,
    r#"; clear 2>/dev/null || printf '\033[3J\033[2J\033[H'"#,
    "\n",
);

/// SSH 客户端回调。首版接受所有主机密钥。
pub struct ClientHandler;

impl client::Handler for ClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &russh::keys::ssh_key::PublicKey,
    ) -> Result<bool, Self::Error> {
        // TODO: 后续应做 known_hosts 校验与指纹确认，避免中间人攻击。
        Ok(true)
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuthMethod {
    Password,
    Key,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectOpts {
    pub id: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth: AuthMethod,
    pub password: Option<String>,
    pub private_key_path: Option<String>,
    pub passphrase: Option<String>,
    pub cols: u32,
    pub rows: u32,
}

pub(crate) fn home_dir() -> Option<PathBuf> {
    env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

pub(crate) fn expand_home_path(path: &str) -> PathBuf {
    if path == "~" {
        return home_dir().unwrap_or_else(|| PathBuf::from(path));
    }
    if path.starts_with("~/") || path.starts_with("~\\") {
        if let Some(home) = home_dir() {
            return home.join(&path[2..]);
        }
    }
    PathBuf::from(path)
}

#[tauri::command]
pub fn default_private_key_path() -> String {
    let candidates = [
        "~/.ssh/id_rsa",
        "~/.ssh/id_ed25519",
        "~/.ssh/id_ecdsa",
        "~/.ssh/id_dsa",
    ];
    candidates
        .iter()
        .copied()
        .find(|path| expand_home_path(path).is_file())
        .unwrap_or("~/.ssh/id_rsa")
        .to_string()
}

#[tauri::command]
pub async fn ssh_connect(
    app: AppHandle,
    state: State<'_, AppState>,
    opts: ConnectOpts,
) -> Result<(), String> {
    connect_inner(app, state, opts)
        .await
        .map_err(|e| e.to_string())
}

async fn connect_inner(
    app: AppHandle,
    state: State<'_, AppState>,
    opts: ConnectOpts,
) -> Result<()> {
    let config = Arc::new(client::Config::default());
    let mut handle =
        client::connect(config, (opts.host.as_str(), opts.port), ClientHandler).await?;

    // 认证。
    let auth_ok = match opts.auth {
        AuthMethod::Password => {
            let pw = opts.password.clone().ok_or_else(|| anyhow!("缺少密码"))?;
            matches!(
                handle
                    .authenticate_password(opts.username.as_str(), pw)
                    .await?,
                AuthResult::Success
            )
        }
        AuthMethod::Key => {
            let path = opts
                .private_key_path
                .clone()
                .ok_or_else(|| anyhow!("缺少私钥路径"))?;
            let expanded_path = expand_home_path(&path);
            if !expanded_path.is_file() {
                return Err(anyhow!("私钥文件不存在：{}", expanded_path.display()));
            }
            let key = load_secret_key(expanded_path, opts.passphrase.as_deref())?;
            let res = handle
                .authenticate_publickey(
                    opts.username.as_str(),
                    PrivateKeyWithHashAlg::new(Arc::new(key), None),
                )
                .await?;
            matches!(res, AuthResult::Success)
        }
    };
    if !auth_ok {
        return Err(anyhow!("认证失败：用户名 / 密码 / 私钥不正确"));
    }

    let handle = Arc::new(handle);

    // 打开交互式 shell 通道。
    let channel = handle.channel_open_session().await?;
    channel
        .request_pty(false, "xterm-256color", opts.cols, opts.rows, 0, 0, &[])
        .await?;
    channel.request_shell(true).await?;

    // 启动 actor 任务桥接通道 <-> 前端。
    let (tx, mut rx) = mpsc::channel::<ShellCmd>(256);
    let id = opts.id.clone();
    let app2 = app.clone();
    tokio::spawn(async move {
        let (mut read_half, write_half) = channel.split();
        // 注入 OSC 7 上报配置，使前端能拿到 shell 的真实 cwd。
        let _ = write_half.data(OSC7_SETUP.as_bytes()).await;
        let out_event = format!("terminal-output-{id}");
        let engine = base64::engine::general_purpose::STANDARD;
        loop {
            tokio::select! {
                msg = read_half.wait() => {
                    match msg {
                        Some(ChannelMsg::Data { data }) => {
                            let _ = app2.emit(&out_event, engine.encode(&data[..]));
                        }
                        Some(ChannelMsg::ExtendedData { data, .. }) => {
                            let _ = app2.emit(&out_event, engine.encode(&data[..]));
                        }
                        Some(ChannelMsg::Eof) | None => break,
                        _ => {}
                    }
                }
                cmd = rx.recv() => {
                    match cmd {
                        Some(ShellCmd::Data(b)) => { let _ = write_half.data(b.as_slice()).await; }
                        Some(ShellCmd::Resize(c, r)) => {
                            let _ = write_half.window_change(c, r, 0, 0).await;
                        }
                        Some(ShellCmd::Close) | None => break,
                    }
                }
            }
        }
        let _ = app2.emit(&format!("terminal-closed-{id}"), ());
    });

    let entry = Arc::new(SessionEntry {
        handle,
        shell_tx: tx,
        sftp: tokio::sync::Mutex::new(None),
    });
    state.sessions.lock().await.insert(opts.id, entry);
    Ok(())
}

/// 向指定会话的 shell actor 发送一条命令。
async fn send_cmd(state: &State<'_, AppState>, id: &str, cmd: ShellCmd) -> Result<(), String> {
    let sessions = state.sessions.lock().await;
    let entry = sessions.get(id).ok_or("会话不存在")?;
    entry
        .shell_tx
        .send(cmd)
        .await
        .map_err(|_| "会话已关闭".to_string())
}

#[tauri::command]
pub async fn ssh_write(state: State<'_, AppState>, id: String, data: String) -> Result<(), String> {
    send_cmd(&state, &id, ShellCmd::Data(data.into_bytes())).await
}

#[tauri::command]
pub async fn ssh_resize(
    state: State<'_, AppState>,
    id: String,
    cols: u32,
    rows: u32,
) -> Result<(), String> {
    send_cmd(&state, &id, ShellCmd::Resize(cols, rows)).await
}

#[tauri::command]
pub async fn ssh_disconnect(state: State<'_, AppState>, id: String) -> Result<(), String> {
    // 先发关闭命令，再从会话表移除（drop 句柄与 SFTP 会话）。
    let _ = send_cmd(&state, &id, ShellCmd::Close).await;
    state.sessions.lock().await.remove(&id);
    Ok(())
}
