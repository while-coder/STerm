# STerm

基于 Tauri + Vue 3 的 SSH / SFTP 终端客户端。

## 已实现

- **SSH 交互式终端**：密码 / 私钥认证，PTY + xterm.js 全功能终端，自适应窗口尺寸。
- **SFTP 文件管理**：浏览目录、进入子目录、上传 / 下载文件（带系统文件对话框）。

## 技术栈

| 层 | 选型 |
|----|------|
| 外壳 | Tauri 2 |
| 前端 | Vue 3 + TypeScript + Vite |
| 终端渲染 | xterm.js（`@xterm/xterm` + fit / web-links addon） |
| SSH | [`russh`](https://crates.io/crates/russh) 0.61（`ring` 加密后端） |
| SFTP | [`russh-sftp`](https://crates.io/crates/russh-sftp) 2.x |

> 注：`russh` 默认使用 `aws-lc-rs` 后端，在 Windows 上需要 NASM 工具链。本项目改用 `ring` 后端规避（见 `src-tauri/Cargo.toml`）。

## 架构

```
前端 (Vue)                         后端 (Rust / Tauri commands)
  TerminalView ──ssh_write/resize──▶ ssh.rs: 每会话一个 actor 任务
       ▲                                持有 Channel.split() 读写两半，
       └──event: terminal-output──────  select! 桥接 PTY 输出与输入命令
  FileBrowser  ──sftp_list/up/down──▶ sftp.rs: 复用 SSH 连接懒加载 sftp 子系统
```

- 终端输出经 base64 编码后通过 Tauri 事件推送，保证二进制流 / 多字节 UTF-8 完整还原。
- 会话状态集中在 `state.rs` 的 `AppState`，按前端生成的 UUID 索引。

## 开发运行

```bash
pnpm install
pnpm tauri dev      # 开发模式（热重载）
pnpm tauri build    # 打包
```

## 后续路线

- [ ] **rzsz (ZMODEM)**：在 PTY 输出流中探测 ZMODEM 启动序列，劫持数据流交给 ZMODEM 状态机处理，完成后还原终端模式。
- [ ] 主机密钥校验（known_hosts）与指纹确认。
- [ ] 多会话标签页。
- [ ] SFTP 传输进度、断点续传、目录递归上传 / 下载。
