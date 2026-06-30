// 对后端 Tauri command 的薄封装，集中类型定义。
import { invoke } from "@tauri-apps/api/core";

export type AuthMethod = "password" | "key";

export interface ConnectOpts {
  id: string;
  host: string;
  port: number;
  username: string;
  auth: AuthMethod;
  password?: string;
  privateKeyPath?: string;
  passphrase?: string;
  cols: number;
  rows: number;
}

export interface SavedConnection {
  id: string;
  label: string;
  host: string;
  port: number;
  username: string;
  auth: AuthMethod;
  privateKeyPath?: string;
  remember: boolean;
  password?: string;
  passphrase?: string;
  /** 分组名（空 / undefined 归入「未分组」）。 */
  group?: string;
  /** 收藏置顶。 */
  favorite?: boolean;
}

export interface FileEntry {
  name: string;
  path: string;
  isDir: boolean;
  size: number;
  modified: number;
}

/** 导出文件中的连接条目：在 SavedConnection 基础上可携带私钥文件内容。 */
export type ExportedConnection = SavedConnection & { privateKey?: string };

export const sshConnect = (opts: ConnectOpts) => invoke<void>("ssh_connect", { opts });
export const sshWrite = (id: string, data: string) => invoke<void>("ssh_write", { id, data });
export const sshResize = (id: string, cols: number, rows: number) =>
  invoke<void>("ssh_resize", { id, cols, rows });
export const sshDisconnect = (id: string) => invoke<void>("ssh_disconnect", { id });
export const defaultPrivateKeyPath = () => invoke<string>("default_private_key_path");

// —— 导入 / 导出文件读写 ——
export const readTextFile = (path: string) => invoke<string>("read_text_file", { path });
export const writeTextFile = (path: string, contents: string) =>
  invoke<void>("write_text_file", { path, contents });
/** 把导入的私钥内容写到 ~/.ssh/sterm-keys/<id>，返回落盘后的路径。 */
export const importPrivateKey = (id: string, contents: string) =>
  invoke<string>("import_private_key", { id, contents });

// —— 系统凭证中的主密码 ——
export const getMasterPassword = () => invoke<string | null>("get_master_password");
export const setMasterPassword = (password: string) =>
  invoke<void>("set_master_password", { password });
export const deleteMasterPassword = () => invoke<void>("delete_master_password");

export const sftpHome = (id: string) => invoke<string>("sftp_home", { id });
export const sftpList = (id: string, path: string) =>
  invoke<FileEntry[]>("sftp_list", { id, path });
export const sftpDownload = (
  id: string,
  remotePath: string,
  localPath: string,
  transferId: string,
  expectedSize?: number
) => invoke<void>("sftp_download", { id, remotePath, localPath, transferId, expectedSize });
export const sftpDownloadDir = (
  id: string,
  remotePath: string,
  localPath: string,
  transferId: string
) => invoke<void>("sftp_download_dir", { id, remotePath, localPath, transferId });
export const sftpUpload = (
  id: string,
  localPath: string,
  remotePath: string,
  transferId: string
) => invoke<void>("sftp_upload", { id, localPath, remotePath, transferId });
export const sftpCancel = (transferId: string) =>
  invoke<void>("sftp_cancel", { transferId });
export const sftpMkdir = (id: string, path: string) =>
  invoke<void>("sftp_mkdir", { id, path });
export const sftpRename = (id: string, from: string, to: string) =>
  invoke<void>("sftp_rename", { id, from, to });
export const sftpRemove = (id: string, path: string, isDir: boolean) =>
  invoke<void>("sftp_remove", { id, path, isDir });
export const ensureDir = (path: string) => invoke<void>("ensure_dir", { path });
export const openDir = (path: string) => invoke<void>("open_dir", { path });
export const openLocalPath = (path: string) => invoke<void>("open_local_path", { path });
