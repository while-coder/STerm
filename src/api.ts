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

export interface FileEntry {
  name: string;
  path: string;
  isDir: boolean;
  size: number;
  modified: number;
}

export const sshConnect = (opts: ConnectOpts) => invoke<void>("ssh_connect", { opts });
export const sshWrite = (id: string, data: string) => invoke<void>("ssh_write", { id, data });
export const sshResize = (id: string, cols: number, rows: number) =>
  invoke<void>("ssh_resize", { id, cols, rows });
export const sshDisconnect = (id: string) => invoke<void>("ssh_disconnect", { id });

export const sftpHome = (id: string) => invoke<string>("sftp_home", { id });
export const sftpList = (id: string, path: string) =>
  invoke<FileEntry[]>("sftp_list", { id, path });
export const sftpDownload = (id: string, remotePath: string, localPath: string) =>
  invoke<void>("sftp_download", { id, remotePath, localPath });
export const sftpUpload = (id: string, localPath: string, remotePath: string) =>
  invoke<void>("sftp_upload", { id, localPath, remotePath });
